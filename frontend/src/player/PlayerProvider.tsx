import { createContext, useCallback, useContext, useEffect, useEffectEvent, useRef, type ReactNode } from 'react'
import type { Track } from '../api/generated'
import { artistName } from '../format'
import { usePlayer } from './store'

function shuffled(tracks: Track[]) {
  const result = [...tracks]
  for (let index = result.length - 1; index > 0; index--) {
    const other = Math.floor(Math.random() * (index + 1))
    const track = result[index]
    result[index] = result[other]
    result[other] = track
  }
  return result
}

const AudioContext = createContext<{ play: () => void; playTrack: (track: Track) => void; playTracks: (tracks: Track[]) => void; playQueuedTrack: (track: Track) => void; clearQueue: () => void; playQueue: () => void; shuffleQueue: () => void; cycleRepeat: () => void; removeQueuedTrack: (id: string) => void; previous: () => void; next: () => void; toggle: () => void; seek: (time: number) => void; skip: (seconds: number) => void; reload: () => void } | null>(null)

export function useAudioControls() {
  const controls = useContext(AudioContext)
  if (!controls) throw new Error('PlayerProvider is required')
  return controls
}

export function PlayerProvider({ children }: { children: ReactNode }) {
  const audio = useRef<HTMLAudioElement>(null)
  const pendingPlay = useRef<string | null>(null)
  const track = usePlayer((state) => state.track)
  const repeat = usePlayer((state) => state.repeat)
  const canPrevious = usePlayer((state) => !!state.track && (state.history.length > 0 || state.repeat === 'queue' || state.repeat === 'shuffle'))
  const canNext = usePlayer((state) => !!state.track && (state.queue.length > 0 || state.repeat === 'queue' || state.repeat === 'shuffle'))
  const update = usePlayer((state) => state.update)
  useEffect(() => {
    const element = audio.current
    return () => { element?.pause() }
  }, [track?.id])
  useEffect(() => {
    if (!('mediaSession' in navigator) || typeof MediaMetadata === 'undefined') return
    navigator.mediaSession.metadata = track ? new MediaMetadata({
      title: track.title,
      artist: artistName(track.artistCredits),
    }) : null
    return () => { navigator.mediaSession.metadata = null }
  }, [track])
  const syncTime = () => {
    if (!audio.current) return
    update({
      currentTime: audio.current.currentTime,
      duration: Number.isFinite(audio.current.duration) ? audio.current.duration : 0,
    })
  }

  const play = useCallback(() => {
    const element = audio.current
    if (!element || !track) return
    update({ error: null })
    void element.play().catch((error: unknown) => {
      if (audio.current !== element || usePlayer.getState().track?.id !== track.id) return
      if (error instanceof DOMException && error.name === 'AbortError') return
      if (error instanceof DOMException && error.name === 'NotAllowedError') {
        update({ playing: false })
        return
      }
      update({ playing: false, error: 'Could not play audio. Check the source and your connection, then try again.' })
    })
  }, [track, update])
  const startTrack = (nextTrack: Track) => {
    if (track?.id === nextTrack.id) {
      play()
      return
    }
    pendingPlay.current = nextTrack.id
    usePlayer.getState().select(nextTrack)
  }
  const playTrack = (nextTrack: Track) => {
    update({ history: [], queue: [] })
    startTrack(nextTrack)
  }
  const playTracks = (tracks: Track[]) => {
    const unique = [...new Map(tracks.map((item) => [item.id, item])).values()]
    const first = unique[0]
    if (!first) return
    update({ history: [], queue: unique.slice(1) })
    if (usePlayer.getState().track?.id === first.id && audio.current) {
      audio.current.currentTime = 0
      syncTime()
    }
    startTrack(first)
  }
  const activateQueuedTrack = (nextTrack: Track) => {
    const state = usePlayer.getState()
    const tracks = [...state.history, ...(state.track ? [state.track] : []), ...state.queue]
    const index = tracks.findIndex((item) => item.id === nextTrack.id)
    if (index < 0) return
    update({ history: tracks.slice(0, index), queue: tracks.slice(index + 1) })
    if (state.track?.id === nextTrack.id && audio.current) {
      audio.current.currentTime = 0
      syncTime()
    }
    startTrack(nextTrack)
  }
  const playQueuedTrack = (nextTrack: Track) => {
    const state = usePlayer.getState()
    if (state.track?.id === nextTrack.id) return
    const tracks = [...state.history, ...(state.track ? [state.track] : []), ...state.queue]
    if (!tracks.some((item) => item.id === nextTrack.id)) return
    update({ history: [], queue: tracks.filter((item) => item.id !== nextTrack.id) })
    startTrack(nextTrack)
  }
  const shuffleQueue = () => {
    const state = usePlayer.getState()
    playTracks(shuffled([...state.history, ...(state.track ? [state.track] : []), ...state.queue]))
  }
  const cycleRepeat = () => {
    const state = usePlayer.getState()
    const repeat = state.repeat === 'off' ? 'track' : state.repeat === 'track' ? 'queue' : state.repeat === 'queue' ? 'shuffle' : 'off'
    if (repeat === 'shuffle') {
      update({ repeat, history: [], queue: shuffled([...state.history, ...state.queue]) })
    } else update({ repeat })
  }
  const previous = () => {
    const state = usePlayer.getState()
    if (!state.history.length && state.repeat === 'shuffle') {
      shuffleQueue()
      return
    }
    const previousTrack = state.history.at(-1) ?? (state.repeat === 'queue' ? state.queue.at(-1) ?? state.track : null)
    if (previousTrack) activateQueuedTrack(previousTrack)
  }
  const next = () => {
    const state = usePlayer.getState()
    if (!state.queue.length && state.repeat === 'shuffle') {
      shuffleQueue()
      return
    }
    const nextTrack = state.queue[0] ?? (state.repeat === 'queue' ? state.history[0] ?? state.track : null)
    if (nextTrack) activateQueuedTrack(nextTrack)
  }
  const mediaPrevious = useEffectEvent(() => previous())
  const mediaNext = useEffectEvent(() => next())
  useEffect(() => {
    if (!('mediaSession' in navigator)) return
    const session = navigator.mediaSession
    const handlers: [MediaSessionAction, MediaSessionActionHandler | null][] = [
      ['previoustrack', canPrevious ? () => mediaPrevious() : null],
      ['nexttrack', canNext ? () => mediaNext() : null],
    ]
    const registered: MediaSessionAction[] = []
    for (const [action, handler] of handlers) {
      try {
        session.setActionHandler(action, handler)
        registered.push(action)
      } catch (error) {
        if (!(error instanceof DOMException && error.name === 'NotSupportedError')) throw error
      }
    }
    return () => {
      for (const action of registered) session.setActionHandler(action, null)
    }
  }, [canPrevious, canNext])
  const playQueue = () => {
    const state = usePlayer.getState()
    const first = state.history[0] ?? state.track ?? state.queue[0]
    if (first) activateQueuedTrack(first)
  }
  const removeQueuedTrack = (id: string) => {
    const state = usePlayer.getState()
    if (state.track?.id === id) {
      pendingPlay.current = null
      audio.current?.pause()
      update({ track: null, playing: false, currentTime: 0, duration: 0, ready: false, error: null,
        history: state.history.filter((item) => item.id !== id), queue: state.queue.filter((item) => item.id !== id) })
    } else update({ history: state.history.filter((item) => item.id !== id), queue: state.queue.filter((item) => item.id !== id) })
  }
  const clearQueue = () => {
    pendingPlay.current = null
    audio.current?.pause()
    update({ history: [], queue: [], track: null, playing: false, currentTime: 0, duration: 0, ready: false, error: null })
  }
  useEffect(() => {
    if (track && pendingPlay.current === track.id) {
      pendingPlay.current = null
      play()
    }
  }, [track, play])
  const toggle = () => {
    const element = audio.current
    if (!element || !track) return
    if (!element.paused) element.pause()
    else play()
  }
  const seek = (time: number) => {
    const element = audio.current
    if (!element || !Number.isFinite(element.duration)) return
    element.currentTime = Math.max(0, Math.min(time, element.duration))
    syncTime()
  }
  const skip = (seconds: number) => {
    if (audio.current) seek(audio.current.currentTime + seconds)
  }
  const reload = () => {
    update({ error: null, playing: false, ready: false, currentTime: 0 })
    audio.current?.load()
  }

  return (
    <AudioContext.Provider value={{ play, playTrack, playTracks, playQueuedTrack, clearQueue, playQueue, shuffleQueue, cycleRepeat, removeQueuedTrack, previous, next, toggle, seek, skip, reload }}>
      {children}
      {track && <audio
        key={track.id}
        ref={audio}
        src={track.originalAudioFile.playbackUrl}
        preload="metadata"
        loop={repeat === 'track'}
        onLoadedMetadata={syncTime}
        onDurationChange={syncTime}
        onCanPlay={() => update({ ready: true, error: null })}
        onPlay={() => update({ playing: true })}
        onPause={() => update({ playing: false })}
        onEnded={() => {
          const state = usePlayer.getState()
          if (state.queue.length || state.repeat === 'queue' || state.repeat === 'shuffle') next()
          else update({ playing: false })
        }}
        onTimeUpdate={syncTime}
        onError={() => update({ playing: false, ready: false, error: 'Could not load audio. Check the source and your connection.' })}
      />}
    </AudioContext.Provider>
  )
}

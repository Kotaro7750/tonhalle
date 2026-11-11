import { createContext, useCallback, useContext, useEffect, useEffectEvent, useLayoutEffect, useRef, type ReactNode } from 'react'
import { flushSync } from 'react-dom'
import { useQuery } from '@tanstack/react-query'
import { releaseOptions } from '../api/queries'
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
  const wasPlaying = useRef(false)
  const track = usePlayer((state) => state.track)
  const { data: release } = useQuery({ ...releaseOptions(track?.releaseId ?? ''), enabled: !!track })
  const repeat = usePlayer((state) => state.repeat)
  const playing = usePlayer((state) => state.playing)
  const canPrevious = usePlayer((state) => !!state.track && (state.history.length > 0 || state.repeat === 'queue' || state.repeat === 'shuffle'))
  const canNext = usePlayer((state) => !!state.track && (state.queue.length > 0 || state.repeat === 'queue' || state.repeat === 'shuffle'))
  const update = usePlayer((state) => state.update)
  useEffect(() => {
    if (!('mediaSession' in navigator) || typeof MediaMetadata === 'undefined') return
    navigator.mediaSession.metadata = track ? new MediaMetadata({
      title: track.title,
      album: release?.title,
      artist: artistName(track.artistCredits),
      artwork: [{ src: new URL(`${import.meta.env.BASE_URL}media-artwork.png`, window.location.href).href, sizes: '512x512', type: 'image/png' }],
    }) : null
    return () => { navigator.mediaSession.metadata = null }
  }, [track, release?.title])
  useEffect(() => {
    if (!('mediaSession' in navigator)) return
    navigator.mediaSession.playbackState = track ? (playing ? 'playing' : 'paused') : 'none'
    if (!track && 'setPositionState' in navigator.mediaSession) navigator.mediaSession.setPositionState()
  }, [track, playing])
  const syncTime = () => {
    if (!audio.current) return
    const element = audio.current
    if ('mediaSession' in navigator && 'setPositionState' in navigator.mediaSession) {
      navigator.mediaSession.setPositionState(Number.isFinite(element.duration) && element.duration > 0
        ? { duration: element.duration, position: Math.max(0, Math.min(element.currentTime, element.duration)), playbackRate: element.playbackRate }
        : undefined)
    }
    update({
      currentTime: audio.current.currentTime,
      duration: Number.isFinite(audio.current.duration) ? audio.current.duration : 0,
    })
  }

  const syncPlayback = useEffectEvent(() => {
    const element = audio.current
    if (!element) return
    syncTime()
    update({ playing: !element.paused && !element.ended })
  })
  useEffect(() => {
    const restore = () => { if (document.visibilityState === 'visible') syncPlayback() }
    document.addEventListener('visibilitychange', restore)
    window.addEventListener('pageshow', restore)
    return () => {
      document.removeEventListener('visibilitychange', restore)
      window.removeEventListener('pageshow', restore)
      if ('mediaSession' in navigator) {
        navigator.mediaSession.playbackState = 'none'
        if ('setPositionState' in navigator.mediaSession) navigator.mediaSession.setPositionState()
      }
    }
  }, [])

  const play = useCallback(() => {
    const element = audio.current
    const currentTrack = usePlayer.getState().track
    if (!element || !currentTrack) return
    update({ error: null })
    void element.play().catch((error: unknown) => {
      if (audio.current !== element || usePlayer.getState().track?.id !== currentTrack.id) return
      if (error instanceof DOMException && error.name === 'AbortError') return
      if (error instanceof DOMException && error.name === 'NotAllowedError') {
        update({ playing: false })
        return
      }
      update({ playing: false, error: 'Could not play audio. Check the source and your connection, then try again.' })
    })
  }, [update])
  const startTrack = (nextTrack: Track) => {
    if (track?.id === nextTrack.id) {
      play()
      return
    }
    // Commit the source before play() while the browser/OS action still has user activation.
    flushSync(() => usePlayer.getState().select(nextTrack))
    play()
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
  const mediaPlay = useEffectEvent(() => play())
  const mediaPause = useEffectEvent(() => audio.current?.pause())
  const mediaSeek = useEffectEvent((time: number) => seek(time))
  useEffect(() => {
    if (!('mediaSession' in navigator)) return
    const session = navigator.mediaSession
    const handlers: [MediaSessionAction, MediaSessionActionHandler | null][] = [
      ['play', track ? () => mediaPlay() : null],
      ['pause', track ? () => mediaPause() : null],
      // iOS shares these buttons with track navigation; prefer previous/next for music.
      ['seekbackward', null],
      ['seekforward', null],
      ['seekto', track ? (details) => { if (details.seekTime !== undefined) mediaSeek(details.seekTime) } : null],
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
  }, [!!track, canPrevious, canNext])
  const playQueue = () => {
    const state = usePlayer.getState()
    const first = state.history[0] ?? state.track ?? state.queue[0]
    if (first) activateQueuedTrack(first)
  }
  const stop = () => {
    const element = audio.current
    wasPlaying.current = false
    if (!element) return
    element.pause()
    // Release the media resource as well as stopping playback, including remote playback sessions.
    element.removeAttribute('src')
    element.load()
  }
  const removeQueuedTrack = (id: string) => {
    const state = usePlayer.getState()
    if (state.track?.id === id) {
      stop()
      update({ track: null, playing: false, currentTime: 0, duration: 0, ready: false, error: null,
        history: state.history.filter((item) => item.id !== id), queue: state.queue.filter((item) => item.id !== id) })
    } else update({ history: state.history.filter((item) => item.id !== id), queue: state.queue.filter((item) => item.id !== id) })
  }
  const clearQueue = () => {
    stop()
    update({ history: [], queue: [], track: null, playing: false, currentTime: 0, duration: 0, ready: false, error: null })
  }
  useLayoutEffect(() => {
    audio.current?.load()
  }, [track?.id])
  const toggle = () => {
    const element = audio.current
    if (!element || !track) return
    if (!element.paused) element.pause()
    else play()
  }
  const seek = (time: number) => {
    const element = audio.current
    if (!element || !Number.isFinite(time) || !Number.isFinite(element.duration)) return
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
      {/* Keep the media element alive across tracks so iOS retains its playback session. */}
      {track && <audio
        ref={audio}
        src={track.originalAudioFile.playbackUrl}
        preload="metadata"
        loop={repeat === 'track'}
        onLoadedMetadata={syncTime}
        onDurationChange={syncTime}
        onCanPlay={() => update({ ready: true, error: null })}
        onPlay={() => { wasPlaying.current = true; update({ playing: true }) }}
        onPause={(event) => {
          // A natural ending pauses before ended; a paused seek to the end must not start the queue.
          if (!event.currentTarget.ended) wasPlaying.current = false
          update({ playing: false })
        }}
        onEnded={() => {
          if (!wasPlaying.current) { update({ playing: false }); return }
          wasPlaying.current = false
          const state = usePlayer.getState()
          if (state.queue.length || state.repeat === 'queue' || state.repeat === 'shuffle') next()
          else update({ playing: false })
        }}
        onTimeUpdate={syncTime}
        onSeeked={syncTime}
        onEmptied={syncTime}
        onRateChange={syncTime}
        onError={() => update({ playing: false, ready: false, error: 'Could not load audio. Check the source and your connection.' })}
      />}
    </AudioContext.Provider>
  )
}

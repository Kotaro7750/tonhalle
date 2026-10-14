import { createContext, useCallback, useContext, useEffect, useRef, type ReactNode } from 'react'
import type { Track } from '../api/generated'
import { artistName } from '../format'
import { usePlayer } from './store'

const AudioContext = createContext<{ play: () => void; playTrack: (track: Track) => void; toggle: () => void; seek: (time: number) => void; skip: (seconds: number) => void; reload: () => void } | null>(null)

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
  const playTrack = (nextTrack: Track) => {
    if (track?.id === nextTrack.id) {
      play()
      return
    }
    pendingPlay.current = nextTrack.id
    usePlayer.getState().select(nextTrack)
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
    <AudioContext.Provider value={{ play, playTrack, toggle, seek, skip, reload }}>
      {children}
      {track && <audio
        key={track.id}
        ref={audio}
        src={track.originalAudioFile.playbackUrl}
        preload="metadata"
        loop={repeat}
        onLoadedMetadata={syncTime}
        onDurationChange={syncTime}
        onCanPlay={() => update({ ready: true, error: null })}
        onPlay={() => update({ playing: true })}
        onPause={() => update({ playing: false })}
        onEnded={() => update({ playing: false })}
        onTimeUpdate={syncTime}
        onError={() => update({ playing: false, ready: false, error: 'Could not load audio. Check the source and your connection.' })}
      />}
    </AudioContext.Provider>
  )
}

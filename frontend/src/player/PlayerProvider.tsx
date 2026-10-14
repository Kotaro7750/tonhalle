import { createContext, useCallback, useContext, useEffect, useRef, type ReactNode } from 'react'
import { usePlayer } from './store'

const AudioContext = createContext<{ play: () => void; toggle: () => void; seek: (time: number) => void; reload: () => void } | null>(null)

export function useAudioControls() {
  const controls = useContext(AudioContext)
  if (!controls) throw new Error('PlayerProvider is required')
  return controls
}

export function PlayerProvider({ children }: { children: ReactNode }) {
  const audio = useRef<HTMLAudioElement>(null)
  const track = usePlayer((state) => state.track)
  const update = usePlayer((state) => state.update)
  useEffect(() => {
    const element = audio.current
    return () => { element?.pause() }
  }, [track?.id])
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
      update({ playing: false, error: '音源を再生できませんでした。配信先や通信状況を確認し、もう一度お試しください。' })
    })
  }, [track, update])
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
  const reload = () => {
    update({ error: null, playing: false, ready: false, currentTime: 0 })
    audio.current?.load()
  }

  return (
    <AudioContext.Provider value={{ play, toggle, seek, reload }}>
      {children}
      {track && <audio
        key={track.id}
        ref={audio}
        src={track.originalAudioFile.playbackUrl}
        preload="metadata"
        onLoadedMetadata={syncTime}
        onDurationChange={syncTime}
        onCanPlay={() => update({ ready: true, error: null })}
        onPlay={() => update({ playing: true })}
        onPause={() => update({ playing: false })}
        onEnded={() => update({ playing: false })}
        onTimeUpdate={syncTime}
        onError={() => update({ playing: false, ready: false, error: '音源を読み込めませんでした。配信先や通信状況を確認してください。' })}
      />}
    </AudioContext.Provider>
  )
}

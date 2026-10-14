import { create } from 'zustand'
import type { Track } from '../api/generated'

interface PlayerState {
  track: Track | null
  playing: boolean
  currentTime: number
  duration: number
  ready: boolean
  error: string | null
  select: (track: Track) => void
  update: (state: Partial<Omit<PlayerState, 'select' | 'update'>>) => void
}

export const usePlayer = create<PlayerState>((set) => ({
  track: null, playing: false, currentTime: 0, duration: 0, ready: false, error: null,
  select: (track) => set((state) => state.track?.id === track.id ? state : {
    track, playing: false, currentTime: 0, duration: 0, ready: false, error: null,
  }),
  update: (state) => set(state),
}))

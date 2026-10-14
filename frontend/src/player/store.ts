import { create } from 'zustand'
import type { Track } from '../api/generated'

interface PlayerState {
  track: Track | null
  history: Track[]
  queue: Track[]
  queueOpen: boolean
  enqueue: (track: Track) => void
  enqueueMany: (tracks: Track[]) => void
  moveQueuedTrack: (id: string, index: number) => void
  repeat: 'off' | 'track' | 'queue' | 'shuffle'
  playing: boolean
  currentTime: number
  duration: number
  ready: boolean
  error: string | null
  select: (track: Track) => void
  update: (state: Partial<Omit<PlayerState, 'select' | 'update' | 'enqueue' | 'enqueueMany' | 'moveQueuedTrack'>>) => void
}

export const usePlayer = create<PlayerState>((set) => ({
  track: null, repeat: 'off', playing: false, currentTime: 0, duration: 0, ready: false, error: null,
  history: [], queue: [], queueOpen: false,
  enqueue: (track) => set((state) => state.track?.id === track.id || state.history.some((item) => item.id === track.id) || state.queue.some((item) => item.id === track.id)
    ? state : { queue: [...state.queue, track] }),
  enqueueMany: (tracks) => set((state) => {
    const ids = new Set([state.track?.id, ...state.history.map((item) => item.id), ...state.queue.map((item) => item.id)])
    const added = tracks.filter((item) => {
      if (ids.has(item.id)) return false
      ids.add(item.id)
      return true
    })
    return added.length ? { queue: [...state.queue, ...added] } : state
  }),
  moveQueuedTrack: (id, index) => set((state) => {
    const tracks = [...state.history, ...(state.track ? [state.track] : []), ...state.queue]
    const from = tracks.findIndex((item) => item.id === id)
    if (from < 0 || index < 0 || index >= tracks.length || from === index) return state
    const [item] = tracks.splice(from, 1)
    tracks.splice(index, 0, item)
    const current = state.track ? tracks.findIndex((item) => item.id === state.track?.id) : -1
    return { history: current < 0 ? [] : tracks.slice(0, current), queue: tracks.slice(current + 1) }
  }),
  select: (track) => set((state) => state.track?.id === track.id ? state : {
    history: state.history.filter((item) => item.id !== track.id),
    queue: state.queue.filter((item) => item.id !== track.id),
    track, playing: false, currentTime: 0, duration: 0, ready: false, error: null,
  }),
  update: (state) => set(state),
}))

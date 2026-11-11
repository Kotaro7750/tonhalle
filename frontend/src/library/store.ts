import { create } from 'zustand'
import { persist } from 'zustand/middleware'

type LibraryView = 'releases' | 'artists'

export const useLibrary = create(persist<{
  view: LibraryView
  setView: (view: LibraryView) => void
}>((set) => ({
  view: 'releases',
  setView: (view) => set({ view }),
}), {
  name: 'tonhalle-library',
}))

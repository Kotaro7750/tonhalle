import { queryOptions, infiniteQueryOptions, type QueryClient } from '@tanstack/react-query'
import type { TrackSummary } from './generated'
import { api, ApiError } from './client'

export const retryRequest = (failureCount: number, error: Error) =>
  !(error instanceof ApiError && error.status < 500) && failureCount < 2

export const releasesByArtistOptions = (artistId?: string) => infiniteQueryOptions({
  queryKey: artistId ? ['releases', { artistId }] : ['releases'],
  initialPageParam: undefined as string | undefined,
  queryFn: ({ pageParam, signal }) => api.listReleases(pageParam, signal, artistId),
  getNextPageParam: (page) => page.nextCursor,
})
export const releaseOptions = (id: string) => queryOptions({
  queryKey: ['release', id], queryFn: ({ signal }) => api.getRelease(id, signal),
})
export const trackOptions = (id: string) => queryOptions({
  queryKey: ['track', id], queryFn: ({ signal }) => api.getTrack(id, signal),
})

export const artistCollectionOptions = (artistId: string, client: QueryClient) => queryOptions({
  queryKey: ['artist-release-tracks', artistId],
  queryFn: async ({ signal }) => {
    const tracks: TrackSummary[] = []
    const seen = new Set<string>()
    let cursor: string | undefined
    // Counts and bulk actions include every page, not only the visible release cards.
    do {
      const page = await api.listReleases(cursor, signal, artistId)
      for (const item of page.items) {
        if (seen.has(item.id)) continue
        seen.add(item.id)
        const release = await client.fetchQuery(releaseOptions(item.id))
        tracks.push(...[...release.tracks].sort((a, b) => a.discNumber - b.discNumber || a.trackNumber - b.trackNumber))
      }
      cursor = page.nextCursor
    } while (cursor)
    return { releaseCount: seen.size, tracks }
  },
})

export const releaseListOptions = releasesByArtistOptions()
export const artistListOptions = infiniteQueryOptions({
  queryKey: ['artists'],
  initialPageParam: undefined as string | undefined,
  queryFn: ({ pageParam, signal }) => api.listArtists(pageParam, signal),
  getNextPageParam: (page) => page.nextCursor,
})
export const artistOptions = (id: string) => queryOptions({
  queryKey: ['artist', id], queryFn: ({ signal }) => api.getArtist(id, signal),
})

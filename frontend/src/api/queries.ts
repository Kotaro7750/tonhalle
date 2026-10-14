import { queryOptions, infiniteQueryOptions } from '@tanstack/react-query'
import { api, ApiError } from './client'

export const retryRequest = (failureCount: number, error: Error) =>
  !(error instanceof ApiError && error.status < 500) && failureCount < 2

export const releaseListOptions = infiniteQueryOptions({
  queryKey: ['releases'],
  initialPageParam: undefined as string | undefined,
  queryFn: ({ pageParam, signal }) => api.listReleases(pageParam, signal),
  getNextPageParam: (page) => page.nextCursor,
})
export const releaseOptions = (id: string) => queryOptions({
  queryKey: ['release', id], queryFn: ({ signal }) => api.getRelease(id, signal),
})
export const trackOptions = (id: string) => queryOptions({
  queryKey: ['track', id], queryFn: ({ signal }) => api.getTrack(id, signal),
})

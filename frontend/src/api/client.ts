import { ArtistsApi, Configuration, ContentAccessApi, ReleasesApi, TracksApi, ResponseError } from './generated'
import { authEnabled, getAccessToken } from '../auth/config'

const configuration = new Configuration({
  basePath: (import.meta.env.VITE_API_BASE_URL || '/api/v0').replace(/\/$/, ''),
  credentials: 'include',
  middleware: authEnabled ? [{
    pre: async ({ url, init }) => {
      const token = await getAccessToken()
      if (!token) throw new ApiError('Your session has expired. Please sign in again.', 401)
      const headers = new Headers(init.headers)
      headers.set('Authorization', `Bearer ${token}`)
      return { url, init: { ...init, headers } }
    },
  }] : [],
})
const artists = new ArtistsApi(configuration)
const releases = new ReleasesApi(configuration)
const tracks = new TracksApi(configuration)
const contentAccess = new ContentAccessApi(configuration)

export class ApiError extends Error {
  constructor(message: string, readonly status: number) {
    super(message)
    this.name = 'ApiError'
  }
}

async function request<T>(operation: () => Promise<T>, fallback = 'Could not load music information.'): Promise<T> {
  try {
    return await operation()
  } catch (error) {
    if (!(error instanceof ResponseError)) throw error
    const response = error.response
    let message = response.status === 401 ? 'Your session has expired. Please sign in again.'
      : response.status === 404 ? 'The requested music was not found.' : fallback
    try {
      const problem: unknown = await response.clone().json()
      if (problem && typeof problem === 'object') {
        if ('detail' in problem && typeof problem.detail === 'string') message = problem.detail
        else if ('title' in problem && typeof problem.title === 'string') message = problem.title
      }
    } catch { /* A proxy or offline server may return a non-JSON error. */ }
    throw new ApiError(message, response.status)
  }
}

export const api = {
  configureContentAccess: () => request(() => contentAccess.configureContentAccess(), 'Could not configure content access.'),
  clearContentAccess: () => request(() => contentAccess.clearContentAccess(), 'Could not clear content access.'),
  listArtists: (cursor: string | undefined, signal: AbortSignal) =>
    request(() => artists.listArtists({ cursor, limit: 30 }, { signal })),
  getArtist: (artistId: string, signal: AbortSignal) =>
    request(() => artists.getArtist({ artistId }, { signal })),
  listReleases: (cursor: string | undefined, signal: AbortSignal, artistId?: string) =>
    request(() => releases.listReleases({ cursor, limit: 30, artistId }, { signal })),
  getRelease: (releaseId: string, signal: AbortSignal) =>
    request(() => releases.getRelease({ releaseId }, { signal })),
  getTrack: (trackId: string, signal: AbortSignal) =>
    request(() => tracks.getTrack({ trackId }, { signal })),
}

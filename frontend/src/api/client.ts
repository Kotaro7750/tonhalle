import { Configuration, ReleasesApi, TracksApi, ResponseError } from './generated'

const configuration = new Configuration({
  basePath: (import.meta.env.VITE_API_BASE_URL || '/api/v0').replace(/\/$/, ''),
})
const releases = new ReleasesApi(configuration)
const tracks = new TracksApi(configuration)

export class ApiError extends Error {
  constructor(message: string, readonly status: number) {
    super(message)
    this.name = 'ApiError'
  }
}

async function request<T>(operation: () => Promise<T>): Promise<T> {
  try {
    return await operation()
  } catch (error) {
    if (!(error instanceof ResponseError)) throw error
    const response = error.response
    let message = response.status === 404 ? '指定された音楽が見つかりません。' : '音楽の情報を取得できませんでした。'
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
  listReleases: (cursor: string | undefined, signal: AbortSignal) =>
    request(() => releases.listReleases({ cursor, limit: 30 }, { signal })),
  getRelease: (releaseId: string, signal: AbortSignal) =>
    request(() => releases.getRelease({ releaseId }, { signal })),
  getTrack: (trackId: string, signal: AbortSignal) =>
    request(() => tracks.getTrack({ trackId }, { signal })),
}

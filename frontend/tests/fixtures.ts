import type { Page, Route } from '@playwright/test'
import type { ArtistCredit, Release, ReleaseSummary, Track } from '../src/api/generated'

export const releaseId = `release-${'a'.repeat(64)}`
export const trackId = `track-${'b'.repeat(64)}`
export const secondTrackId = `track-${'c'.repeat(64)}`
export const credits: ArtistCredit[] = [
  { artistId: `artist-${'d'.repeat(64)}`, name: 'Alice', joinPhrase: ' & ' },
  { artistId: `artist-${'e'.repeat(64)}`, name: 'Bob', joinPhrase: null },
]
export const summary: ReleaseSummary = { id: releaseId, title: 'Evening Sessions', releaseDate: '2026-10', artistCredits: credits }
export const track: Track = {
  id: trackId, releaseId, title: 'First Light', discNumber: 1, trackNumber: 1,
  durationMs: 15000, artistCredits: credits,
  originalAudioFile: {
    id: '2f1c75fc-55de-4d4b-8992-76050f65dff0', mediaType: 'audio/wav',
    playbackUrl: 'http://127.0.0.1:5173/test-audio.wav',
    sampleRate: 8000, bitDepth: 16, channels: 1, bitrate: 128000,
  },
}
export const release: Release = {
  ...summary,
  tracks: [
    track,
    { ...track, id: secondTrackId, discNumber: 2, title: 'Second Light' },
  ],
}

// A real PCM WAV lets the browser exercise native loading, playback and seeking.
export function wav(): Buffer {
  const frames = 8000 * 15
  const bytes = frames * 2
  const buffer = Buffer.alloc(44 + bytes)
  buffer.write('RIFF', 0)
  buffer.writeUInt32LE(36 + bytes, 4)
  buffer.write('WAVEfmt ', 8)
  buffer.writeUInt32LE(16, 16)
  buffer.writeUInt16LE(1, 20)
  buffer.writeUInt16LE(1, 22)
  buffer.writeUInt32LE(8000, 24)
  buffer.writeUInt32LE(16000, 28)
  buffer.writeUInt16LE(2, 32)
  buffer.writeUInt16LE(16, 34)
  buffer.write('data', 36)
  buffer.writeUInt32LE(bytes, 40)
  return buffer
}

export function serveWav(route: Route) {
  const body = wav()
  const range = route.request().headers().range?.match(/^bytes=(\d+)-(\d*)$/)
  const headers = { 'Accept-Ranges': 'bytes' }
  if (!range) return route.fulfill({ contentType: 'audio/wav', headers, body })
  const start = Number(range[1])
  const end = range[2] ? Math.min(Number(range[2]), body.length - 1) : body.length - 1
  return route.fulfill({
    status: 206, contentType: 'audio/wav',
    headers: { ...headers, 'Content-Range': `bytes ${start}-${end}/${body.length}` },
    body: body.subarray(start, end + 1),
  })
}

export async function mockLibrary(page: Page) {
  await page.route('**/api/v0/**', async (route) => {
    const path = new URL(route.request().url()).pathname
    if (path === '/api/v0/releases') return route.fulfill({ json: { items: [summary] } })
    if (path === `/api/v0/releases/${releaseId}`) return route.fulfill({ json: release })
    if (path === `/api/v0/tracks/${trackId}`) return route.fulfill({ json: track })
    if (path === `/api/v0/tracks/${secondTrackId}`) return route.fulfill({ json: { ...track, id: secondTrackId, title: 'Second Light' } })
    return route.fulfill({ status: 404, contentType: 'application/problem+json', json: { type: 'about:blank', title: 'Not Found', status: 404 } })
  })
  await page.route('**/test-audio.wav', serveWav)
}

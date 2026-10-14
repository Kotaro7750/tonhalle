import { readFileSync } from 'node:fs'
import { test, expect } from '@playwright/test'
import { mockLibrary, track, trackId } from './fixtures'

for (const [extension, mediaType, label, bitDepth] of [
  ['flac', 'audio/flac', 'FLAC', 16],
  ['aac', 'audio/aac', 'AAC', null],
  ['m4a', 'audio/mp4', 'M4A', null],
  ['ogg', 'audio/ogg', 'OGG', null],
  ['opus', 'audio/ogg', 'OGG', null],
  ['mp3', 'audio/mpeg', 'MP3', null],
] as const) {
  test(`plays ${extension} audio and displays its format without inventing bit depth`, async ({ page }) => {
    await mockLibrary(page)
    const playbackUrl = `http://127.0.0.1:5173/test-audio.${extension}`
    await page.route(`**/api/v0/tracks/${trackId}`, (route) => route.fulfill({
      json: { ...track, originalAudioFile: { ...track.originalAudioFile, mediaType, bitDepth, playbackUrl } },
    }))
    const bytes = readFileSync(new URL(`../../test_fixtures/tone.${extension}`, import.meta.url))
    await page.route(`**/test-audio.${extension}`, (route) => route.fulfill({ contentType: mediaType, body: bytes }))
    await page.goto(`/#/tracks/${trackId}`)
    const player = page.getByRole('dialog', { name: 'Player' })
    await expect(player.getByText(label, { exact: true })).toBeVisible()
    if (bitDepth === null) await expect(player.getByText(/ bit/)).toHaveCount(0)
    else await expect(player.getByText(/16 bit/)).toBeVisible()
    await expect(player.getByRole('button', { name: 'Play', exact: true })).toBeEnabled()
    await player.getByRole('button', { name: 'Play', exact: true }).click()
    await expect.poll(() => page.locator('audio').evaluate((audio: HTMLAudioElement) => audio.currentTime)).toBeGreaterThan(0)
    await expect(page.getByRole('alert')).toHaveCount(0)
  })
}

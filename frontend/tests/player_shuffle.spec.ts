import { test, expect, type Page } from '@playwright/test'
import { mockLibrary, release, releaseId, track } from './fixtures'

async function prepareQueue(page: Page) {
  await page.emulateMedia({ reducedMotion: 'reduce' })
  await mockLibrary(page)
  const third = { ...track, id: `track-${'f'.repeat(64)}`, title: 'Third Light', trackNumber: 3 }
  await page.route(`**/api/v0/releases/${releaseId}`, (route) => route.fulfill({ json: { ...release, tracks: [...release.tracks, third] } }))
  await page.route(`**/api/v0/tracks/${third.id}`, (route) => route.fulfill({ json: third }))
  await page.goto(`/#/releases/${releaseId}`)
  for (const title of ['First Light', 'Second Light', 'Third Light']) {
    await page.getByRole('button', { name: `Add to queue: ${title}`, exact: true }).click()
  }
  // A zero draw rotates a three-track Fisher-Yates shuffle by one place.
  await page.evaluate(() => { Math.random = () => 0 })
}

for (const surface of ['mini', 'queue'] as const) {
  test(`shuffle playback from the ${surface} randomizes all tracks and starts the first`, async ({ page }) => {
    await prepareQueue(page)
    const mini = page.getByRole('complementary', { name: 'Now playing' })
    if (surface === 'queue') await mini.getByRole('button', { name: 'Open queue' }).click()
    const controls = surface === 'queue' ? page.getByRole('dialog', { name: 'Queue', exact: true }) : mini
    await controls.getByRole('button', { name: 'Shuffle and play queue' }).click()
    if (surface === 'mini') await mini.getByRole('button', { name: 'Open queue' }).click()
    const queue = page.getByRole('dialog', { name: 'Queue', exact: true })
    const rows = queue.getByRole('list', { name: 'Queued tracks' }).getByRole('listitem')
    for (const [index, title] of ['Second Light', 'Third Light', 'First Light'].entries()) {
      await expect(rows.nth(index)).toContainText(title)
    }
    await expect(queue.locator('[data-queue-state="current"]')).toContainText('Second Light')
    await expect(page.locator('audio')).toHaveJSProperty('paused', false)
    await queue.getByRole('button', { name: 'Close queue' }).click()
    await mini.getByRole('button', { name: 'Next track' }).click()
    await expect(mini.getByRole('link', { name: 'Open player: Third Light' })).toBeVisible()
  })
}

test('shuffle repeat preserves current audio and reshuffles at every queue boundary', async ({ page }) => {
  await prepareQueue(page)
  const mini = page.getByRole('complementary', { name: 'Now playing' })
  await mini.getByRole('button', { name: 'Play queue from start' }).click()
  await mini.getByRole('button', { name: 'Next track' }).click()
  await expect(mini.getByRole('link', { name: 'Open player: Second Light' })).toBeVisible()
  const audio = page.locator('audio')
  await expect(mini.getByRole('button', { name: 'Pause', exact: true })).toBeVisible()
  await mini.getByRole('button', { name: 'Pause', exact: true }).click()
  await audio.evaluate((element: HTMLAudioElement) => { element.currentTime = 3 })
  const originalAudio = await audio.elementHandle()
  for (let i = 0; i < 3; i++) await mini.getByRole('button', { name: 'Repeat mode' }).click()
  await expect(mini.getByRole('button', { name: 'Repeat mode' })).toHaveAttribute('aria-description', 'Shuffle repeat')
  expect(await originalAudio!.evaluate((element: HTMLAudioElement) => ({ connected: element.isConnected, time: element.currentTime, paused: element.paused, loop: element.loop }))).toEqual({ connected: true, time: 3, paused: true, loop: false })
  const expectOrder = async (titles: string[]) => {
    await mini.getByRole('button', { name: 'Open queue' }).click()
    const queue = page.getByRole('dialog', { name: 'Queue', exact: true })
    const rows = queue.getByRole('list', { name: 'Queued tracks' }).getByRole('listitem')
    await expect(rows).toHaveCount(3)
    for (const [index, title] of titles.entries()) await expect(rows.nth(index)).toContainText(title)
    await expect(queue.locator('[data-queue-state="current"]')).toContainText(titles[0])
    await expect(queue.locator('[data-queue-state="played"]')).toHaveCount(0)
    await queue.getByRole('button', { name: 'Close queue' }).click()
  }
  await expectOrder(['Second Light', 'Third Light', 'First Light'])
  // Previous from the first track starts a freshly shuffled cycle at its first track.
  await mini.getByRole('button', { name: 'Previous track' }).click()
  await expectOrder(['Third Light', 'First Light', 'Second Light'])
  await mini.getByRole('button', { name: 'Next track' }).click()
  await expect(mini.getByRole('link', { name: 'Open player: First Light' })).toBeVisible()
  await mini.getByRole('button', { name: 'Previous track' }).click()
  await expect(mini.getByRole('link', { name: 'Open player: Third Light' })).toBeVisible()
  for (let i = 0; i < 3; i++) await mini.getByRole('button', { name: 'Next track' }).click()
  await expectOrder(['First Light', 'Second Light', 'Third Light'])
  for (let i = 0; i < 2; i++) await mini.getByRole('button', { name: 'Next track' }).click()
  await expect(mini.getByRole('link', { name: 'Open player: Third Light' })).toBeVisible()
  await expect(audio).toHaveJSProperty('paused', false)
  await audio.evaluate((element: HTMLAudioElement) => { element.currentTime = 14.8 })
  await expect(mini.getByRole('link', { name: 'Open player: Second Light' })).toBeVisible()
  await expect(audio).toHaveJSProperty('paused', false)
  await expectOrder(['Second Light', 'Third Light', 'First Light'])
  await mini.getByRole('button', { name: 'Repeat mode' }).click()
  await expect(mini.getByRole('button', { name: 'Repeat mode' })).toHaveAttribute('aria-pressed', 'false')
})

test('shuffle repeat restarts a single track at either boundary and after it ends', async ({ page }) => {
  await mockLibrary(page)
  await page.goto(`/#/releases/${releaseId}`)
  await page.getByRole('button', { name: 'Play First Light', exact: true }).click()
  const mini = page.getByRole('complementary', { name: 'Now playing' })
  const audio = page.locator('audio')
  await expect(audio).toHaveJSProperty('paused', false)
  for (let i = 0; i < 3; i++) await mini.getByRole('button', { name: 'Repeat mode' }).click()
  for (const name of ['Previous track', 'Next track']) {
    await audio.evaluate((element: HTMLAudioElement) => { element.currentTime = 8 })
    await mini.getByRole('button', { name }).click()
    await expect.poll(() => audio.evaluate((element: HTMLAudioElement) => element.currentTime)).toBeLessThan(2)
  }
  await audio.evaluate((element: HTMLAudioElement) => { element.currentTime = 14.8 })
  await expect.poll(() => audio.evaluate((element: HTMLAudioElement) => element.currentTime)).toBeLessThan(2)
  await expect(audio).toHaveJSProperty('paused', false)
})

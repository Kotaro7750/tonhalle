import { test, expect } from '@playwright/test'
import { mockLibrary, releaseId, trackId, secondTrackId, summary, release, track } from './fixtures'

for (const mode of ['light', 'dark'] as const) {
  test(`library, detail and direct routes work in ${mode} mode`, async ({ page }, testInfo) => {
    await page.emulateMedia({ colorScheme: mode })
    await mockLibrary(page)
    const errors: string[] = []
    page.on('pageerror', (error) => errors.push(error.message))
    await page.goto('/')
    await expect(page.locator('html')).toHaveClass(mode)
    await expect(page.getByRole('navigation', { name: 'メインナビゲーション' })).toHaveCount(0)
    await expect(page.getByRole('heading', { name: 'Releases', exact: true })).toBeVisible()
    await expect(page.getByText('Alice & Bob')).toBeVisible()
    await expect(page.getByText('2026-10', { exact: true })).toBeVisible()
    await page.screenshot({ path: testInfo.outputPath(`library-${mode}.png`), fullPage: true })
    await page.getByRole('link', { name: /Evening Sessions/ }).click()
    await expect(page.getByRole('heading', { level: 1 })).toHaveText('Evening Sessions')
    await expect(page.getByRole('heading', { name: 'Disc 2' })).toBeVisible()
    await page.screenshot({ path: testInfo.outputPath(`release-${mode}.png`), fullPage: true })
    await page.getByRole('button', { name: 'Play First Light', exact: true }).click()
    await expect(page.getByRole('dialog', { name: 'Player' }).getByRole('heading', { level: 1 })).toHaveText('First Light')
    await expect(page.getByRole('dialog', { name: 'Player' }).getByRole('button', { name: 'Pause', exact: true })).toBeVisible()
    await page.reload()
    await expect(page.getByRole('dialog', { name: 'Player' }).getByRole('heading', { level: 1 })).toHaveText('First Light')
    await expect(page.getByRole('slider', { name: 'Playback position' })).toBeEnabled()
    expect(await page.locator('audio').evaluate((audio: HTMLAudioElement) => audio.paused)).toBe(true)
    await page.screenshot({ path: testInfo.outputPath(`player-${mode}.png`), fullPage: true })
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true)
    expect(errors).toEqual([])
  })
}

for (const systemMode of ['light', 'dark'] as const) {
  test(`theme toggle persists after reload and reopening a tab with ${systemMode} system mode`, async ({ page, context }) => {
    const savedMode = systemMode === 'light' ? 'dark' : 'light'
    await page.emulateMedia({ colorScheme: systemMode })
    await mockLibrary(page)
    await page.goto('/')
    await expect(page.locator('html')).toHaveClass(systemMode)
    const originalBackground = await page.locator('body').evaluate((body) => getComputedStyle(body).backgroundColor)

    const modeSwitch = page.getByRole('switch', { name: 'Dark mode' })
    await expect(modeSwitch).toBeChecked({ checked: systemMode === 'dark' })
    await modeSwitch.press('Space')
    await expect(modeSwitch).toBeChecked({ checked: savedMode === 'dark' })
    await expect(page.locator('html')).toHaveClass(savedMode)
    expect(await page.locator('body').evaluate((body) => getComputedStyle(body).backgroundColor)).not.toBe(originalBackground)
    expect(await page.evaluate(() => localStorage.getItem('tonhalle-theme'))).toBe(savedMode)

    await page.emulateMedia({ colorScheme: savedMode })
    await page.emulateMedia({ colorScheme: systemMode })
    await expect(page.locator('html')).toHaveClass(savedMode)
    await page.reload()
    await expect(page.locator('html')).toHaveClass(savedMode)
    await expect(modeSwitch).toBeChecked({ checked: savedMode === 'dark' })
    await page.close()

    const reopened = await context.newPage()
    await reopened.emulateMedia({ colorScheme: systemMode })
    await mockLibrary(reopened)
    await reopened.goto('/')
    await expect(reopened.locator('html')).toHaveClass(savedMode)
    await expect(reopened.getByRole('heading', { name: 'Releases', exact: true })).toBeVisible()
    const reopenedSwitch = reopened.getByRole('switch', { name: 'Dark mode' })
    await expect(reopenedSwitch).toBeChecked({ checked: savedMode === 'dark' })
    await reopened.getByTitle('ダークモード', { exact: true }).click()
    await expect(reopenedSwitch).toBeChecked({ checked: systemMode === 'dark' })
    await expect(reopened.locator('html')).toHaveClass(systemMode)
    expect(await reopened.evaluate(() => localStorage.getItem('tonhalle-theme'))).toBe(systemMode)
  })
}

test('automatically paginates with the opaque cursor and stops on the final page', async ({ page }) => {
  const cursor = 'opaque+/= cursor'
  const requests: (string | null)[] = []
  await page.route('**/api/v0/releases?*', (route) => {
    const url = new URL(route.request().url())
    requests.push(url.searchParams.get('cursor'))
    expect(url.searchParams.get('limit')).toBe('30')
    if (url.searchParams.has('cursor')) {
      expect(url.searchParams.get('cursor')).toBe(cursor)
      return route.fulfill({ json: { items: [{ ...summary, id: `release-${'f'.repeat(64)}`, title: 'Final Album' }] } })
    }
    return route.fulfill({ json: {
      items: Array.from({ length: 30 }, (_, index) => ({ ...summary, id: `release-${index.toString(16).padStart(64, '0')}`, title: `Album ${index}` })),
      nextCursor: cursor,
    } })
  })
  await page.goto('/')
  await expect(page.getByRole('heading', { name: 'Album 0', exact: true })).toBeVisible()
  await page.getByRole('button', { name: 'Load more' }).scrollIntoViewIfNeeded()
  await expect(page.getByRole('heading', { name: 'Final Album' })).toBeVisible()
  await expect(page.getByText('All releases loaded')).toBeVisible()
  expect(requests).toEqual([null, cursor])
  await expect(page.getByRole('button', { name: 'Load more' })).toHaveCount(0)
})

test('empty library and unknown routes show useful states', async ({ page }) => {
  await page.route('**/api/v0/releases?*', (route) => route.fulfill({ json: { items: [] } }))
  await page.goto('/')
  await expect(page.getByRole('heading', { name: 'Your library is empty' })).toBeVisible()
  await page.goto('/#/missing')
  await expect(page.getByText('Page not found.')).toBeVisible()
  await page.getByRole('link', { name: 'Back to library' }).click()
  await expect(page.getByRole('heading', { name: 'Your library is empty' })).toBeVisible()
})

test('failed initial request can be retried', async ({ page }) => {
  let recover = false
  await page.route('**/api/v0/releases?*', (route) => recover
    ? route.fulfill({ json: { items: [summary] } })
    : route.fulfill({ status: 400, contentType: 'application/problem+json', json: { type: 'about:blank', title: 'Bad Request', status: 400, detail: 'Invalid cursor' } }))
  await page.goto('/')
  await expect(page.getByRole('alert')).toContainText('Invalid cursor')
  recover = true
  await page.getByRole('button', { name: 'Retry', exact: true }).click()
  await expect(page.getByRole('heading', { name: summary.title })).toBeVisible()
})

test('failed next page keeps the existing page and supports retry', async ({ page }) => {
  let recover = false
  await page.route('**/api/v0/releases?*', (route) => {
    if (!new URL(route.request().url()).searchParams.has('cursor')) {
      return route.fulfill({ json: { items: [summary], nextCursor: 'next' } })
    }
    return recover
      ? route.fulfill({ json: { items: [] } })
      : route.fulfill({ status: 400, json: { type: 'about:blank', title: 'Bad Request', status: 400 } })
  })
  await page.goto('/')
  await expect(page.getByRole('alert')).toBeVisible()
  await expect(page.getByRole('heading', { name: summary.title })).toBeVisible()
  recover = true
  await page.getByRole('button', { name: 'Retry', exact: true }).click()
  await expect(page.getByText('All releases loaded')).toBeVisible()
})

test('404 and 400 detail errors are shown without automatic retries', async ({ page }) => {
  await page.route('**/api/v0/releases?*', (route) => route.fulfill({ json: { items: [] } }))
  for (const [path, status] of [[`releases/${releaseId}`, 404], [`tracks/${trackId}`, 400]] as const) {
    let count = 0
    await page.route(`**/api/v0/${path}`, (route) => {
      count++
      return route.fulfill({ status, contentType: 'application/problem+json', json: { type: 'about:blank', title: 'Request failed', status } })
    })
    await page.goto(`/#/${path}`)
    await expect(page.getByRole('alert')).toContainText('Request failed')
    expect(count).toBe(1)
  }
})

test('network failures can recover', async ({ page }) => {
  let recover = false
  await page.route('**/api/v0/releases?*', (route) => recover
    ? route.fulfill({ json: { items: [summary] } }) : route.abort('failed'))
  await page.goto('/')
  await expect(page.getByRole('alert')).toContainText('Check your connection')
  recover = true
  await page.getByRole('button', { name: 'Retry', exact: true }).click()
  await expect(page.getByRole('heading', { name: summary.title })).toBeVisible()
})

test('plays, pauses, seeks and switches tracks while retaining playback on navigation', async ({ page }) => {
  await mockLibrary(page)
  await page.goto(`/#/tracks/${trackId}`)
  const slider = page.getByRole('slider', { name: 'Playback position' })
  await expect(slider).toBeEnabled()
  await page.getByRole('dialog', { name: 'Player' }).getByRole('button', { name: 'Play', exact: true }).click()
  await expect(page.getByRole('dialog', { name: 'Player' }).getByRole('button', { name: 'Pause', exact: true })).toBeVisible()
  await expect.poll(() => page.locator('audio').evaluate((audio: HTMLAudioElement) => audio.currentTime)).toBeGreaterThan(0.2)
  await page.getByRole('dialog', { name: 'Player' }).getByRole('button', { name: 'Pause', exact: true }).click()
  expect(await page.locator('audio').evaluate((audio: HTMLAudioElement) => audio.paused)).toBe(true)
  await slider.focus()
  await page.keyboard.press('Home')
  await page.keyboard.press('ArrowRight')
  await expect.poll(() => page.locator('audio').evaluate((audio: HTMLAudioElement) => audio.currentTime)).toBeGreaterThan(0)
  await page.getByRole('dialog', { name: 'Player' }).getByRole('button', { name: 'Play', exact: true }).click()
  await page.getByRole('dialog', { name: 'Player' }).getByRole('button', { name: 'Close player' }).click()
  await expect(page.getByRole('dialog', { name: 'Player' })).toHaveCount(0)
  await page.getByRole('link', { name: /Evening Sessions/ }).click()
  await expect(page.getByRole('complementary', { name: 'Now playing' })).toBeVisible()
  expect(await page.locator('audio').evaluate((audio: HTMLAudioElement) => audio.paused)).toBe(false)
  const previousAudio = await page.locator('audio').elementHandle()
  await page.getByRole('button', { name: 'Play Second Light', exact: true }).click()
  await expect(page.getByRole('dialog', { name: 'Player' }).getByRole('heading', { level: 1 })).toHaveText('Second Light')
  await expect(page.locator('audio')).toHaveCount(1)
  await expect(page.getByRole('slider', { name: 'Playback position' })).toBeEnabled()
  await expect(page.getByRole('dialog', { name: 'Player' }).getByRole('button', { name: 'Pause', exact: true })).toBeVisible()
  await expect.poll(() => page.locator('audio').evaluate((audio: HTMLAudioElement) => audio.currentTime)).toBeGreaterThan(0)
  expect(await previousAudio!.evaluate((audio: HTMLAudioElement) => audio.paused)).toBe(true)
  expect(page.url()).toContain(secondTrackId)
})

test('disc groups keep track rows inert and play buttons start or resume playback', async ({ page }) => {
  await mockLibrary(page)
  await page.goto(`/#/releases/${releaseId}`)
  await expect(page.getByRole('link', { name: 'Releases', exact: true })).toBeVisible()
  await expect(page.getByRole('heading', { name: 'Tracks', exact: true })).toBeVisible()
  await expect(page.getByText('2 tracks · 0:30')).toBeVisible()
  const firstDisc = page.getByRole('region', { name: 'Disc 1' })
  const secondDisc = page.getByRole('region', { name: 'Disc 2' })
  await expect(firstDisc.getByText('First Light', { exact: true })).toBeVisible()
  await expect(firstDisc.getByText('Second Light', { exact: true })).toHaveCount(0)
  await expect(secondDisc.getByText('Second Light', { exact: true })).toBeVisible()
  await firstDisc.getByText('First Light', { exact: true }).click()
  expect(page.url()).toContain(`/releases/${releaseId}`)
  await expect(page.locator('audio')).toHaveCount(0)

  await firstDisc.getByRole('button', { name: 'Play First Light', exact: true }).press('Enter')
  await expect(page.getByRole('dialog', { name: 'Player' }).getByRole('heading', { level: 1 })).toHaveText('First Light')
  await expect.poll(() => page.locator('audio').evaluate((audio: HTMLAudioElement) => audio.currentTime)).toBeGreaterThan(0.2)
  await page.getByRole('dialog', { name: 'Player' }).getByRole('button', { name: 'Pause', exact: true }).click()
  await page.getByRole('dialog', { name: 'Player' }).getByRole('button', { name: 'Close player' }).click()
  await page.getByRole('button', { name: 'Play First Light', exact: true }).click()
  await expect(page.getByRole('dialog', { name: 'Player' }).getByRole('button', { name: 'Pause', exact: true })).toBeVisible()
  expect(await page.locator('audio').evaluate((audio: HTMLAudioElement) => audio.paused)).toBe(false)

  await page.getByRole('dialog', { name: 'Player' }).getByRole('button', { name: 'Close player' }).click()
  await page.getByRole('button', { name: 'Play First Light', exact: true }).click()
  await expect(page.getByRole('dialog', { name: 'Player' }).getByRole('button', { name: 'Pause', exact: true })).toBeVisible()
  await expect(page.locator('audio')).toHaveCount(1)
})

test('single-disc releases show a disc heading and blocked autoplay can be started manually', async ({ page }) => {
  await page.addInitScript(() => {
    const originalPlay = HTMLMediaElement.prototype.play
    let blockOnce = true
    HTMLMediaElement.prototype.play = function () {
      if (blockOnce) {
        blockOnce = false
        this.setAttribute('data-play-blocked', 'true')
        return Promise.reject(new DOMException('Playback requires user activation', 'NotAllowedError'))
      }
      return originalPlay.call(this)
    }
  })
  await mockLibrary(page)
  await page.route(`**/api/v0/releases/${releaseId}`, (route) => route.fulfill({ json: { ...release, tracks: [track] } }))
  await page.goto(`/#/releases/${releaseId}`)
  await expect(page.getByRole('heading', { name: 'Disc 1' })).toBeVisible()
  await expect(page.getByText('1 track · 0:15')).toBeVisible()
  await page.getByRole('button', { name: 'Play First Light', exact: true }).click()
  await expect(page.locator('audio')).toHaveAttribute('data-play-blocked', 'true')
  await expect(page.getByRole('dialog', { name: 'Player' }).getByRole('button', { name: 'Play', exact: true })).toBeEnabled()
  await expect(page.getByRole('alert')).toHaveCount(0)
  await page.getByRole('dialog', { name: 'Player' }).getByRole('button', { name: 'Play', exact: true }).click()
  await expect(page.getByRole('dialog', { name: 'Player' }).getByRole('button', { name: 'Pause', exact: true })).toBeVisible()
  await expect.poll(() => page.locator('audio').evaluate((audio: HTMLAudioElement) => audio.currentTime)).toBeGreaterThan(0)
})

test('media failures are shown and reloading the source recovers', async ({ page }) => {
  await mockLibrary(page)
  await page.route('**/test-audio.wav', (route) => route.fulfill({ status: 404 }))
  await page.goto(`/#/tracks/${trackId}`)
  await expect(page.getByRole('alert')).toContainText('Could not load audio')
  await expect(page.getByRole('dialog', { name: 'Player' }).getByRole('button', { name: 'Play', exact: true })).toBeDisabled()
  await page.unroute('**/test-audio.wav')
  const { serveWav } = await import('./fixtures')
  await page.route('**/test-audio.wav', serveWav)
  await page.getByRole('button', { name: 'Reload audio' }).click()
  await expect(page.getByRole('slider', { name: 'Playback position' })).toBeEnabled()
  await page.getByRole('dialog', { name: 'Player' }).getByRole('button', { name: 'Play', exact: true }).click()
  await expect(page.getByRole('dialog', { name: 'Player' }).getByRole('button', { name: 'Pause', exact: true })).toBeVisible()
})

test('500 responses and non-JSON proxy errors can recover', async ({ page }) => {
  let recover = false
  await page.route('**/api/v0/releases?*', (route) => recover
    ? route.fulfill({ json: { items: [summary] } })
    : route.fulfill({ status: 500, contentType: 'text/html', body: '<h1>Unavailable</h1>' }))
  await page.goto('/')
  await expect(page.getByRole('alert')).toContainText('Could not load music information')
  recover = true
  await page.getByRole('button', { name: 'Retry', exact: true }).click()
  await expect(page.getByRole('heading', { name: summary.title })).toBeVisible()
})

for (const mode of ['light', 'dark'] as const) {
  test(`mini player opens a bottom overlay and keeps playback independent in ${mode} mode`, async ({ page }, testInfo) => {
    await page.emulateMedia({ colorScheme: mode })
    await mockLibrary(page)
    await page.goto(`/#/releases/${releaseId}`)
    await page.getByRole('button', { name: 'Play First Light', exact: true }).click()
    const player = page.getByRole('dialog', { name: 'Player' })
    await expect(player.getByRole('heading', { level: 1 })).toHaveText('First Light')
    await expect(player).toHaveAttribute('aria-modal', 'true')
    await expect(page.locator('main')).toContainText(summary.title)
    await expect(player.getByRole('button', { name: 'Pause', exact: true })).toBeVisible()
    await page.screenshot({ path: testInfo.outputPath(`overlay-${mode}.png`), animations: 'disabled' })
    const bounds = await player.boundingBox()
    expect(bounds!.y).toBeGreaterThan(0)
    expect(Math.abs(bounds!.y + bounds!.height - page.viewportSize()!.height)).toBeLessThan(2)

    await player.getByRole('button', { name: 'Close player' }).click()
    await expect(player).toHaveCount(0)
    expect(page.url()).toContain(`/releases/${releaseId}`)
    expect(await page.locator('audio').evaluate((audio: HTMLAudioElement) => audio.paused)).toBe(false)
    const mini = page.getByRole('complementary', { name: 'Now playing' })
    const openPlayer = mini.getByRole('link', { name: 'Open player: First Light' })
    await expect(openPlayer).toBeFocused()
    await page.screenshot({ path: testInfo.outputPath(`mini-${mode}.png`), animations: 'disabled' })
    await mini.getByRole('button', { name: 'Pause', exact: true }).click()
    await expect(player).toHaveCount(0)
    expect(page.url()).toContain(`/releases/${releaseId}`)
    expect(await page.locator('audio').evaluate((audio: HTMLAudioElement) => audio.paused)).toBe(true)

    await mini.locator('.chakra-linkbox').click({ position: { x: 12, y: 12 } })
    await expect(player.getByRole('heading', { level: 1 })).toHaveText('First Light')
    expect(await page.locator('audio').evaluate((audio: HTMLAudioElement) => audio.paused)).toBe(true)
    await page.keyboard.press('Escape')
    await expect(player).toHaveCount(0)
    await expect(openPlayer).toBeFocused()

    await mini.getByRole('button', { name: 'Play', exact: true }).click()
    await expect(player).toHaveCount(0)
    expect(await page.locator('audio').evaluate((audio: HTMLAudioElement) => audio.paused)).toBe(false)
    await page.getByRole('link', { name: 'Releases', exact: true }).click()
    await expect(page.getByRole('heading', { name: 'Releases', exact: true })).toBeVisible()
    await openPlayer.press('Enter')
    await expect(player.getByRole('heading', { level: 1 })).toHaveText('First Light')
    await expect(page.locator('main')).toContainText('Releases')
    await page.goBack()
    await expect(player).toHaveCount(0)
    await expect(page.getByRole('heading', { name: 'Releases', exact: true })).toBeVisible()
    await page.goForward()
    await expect(player.getByRole('heading', { level: 1 })).toHaveText('First Light')
    await page.mouse.click(5, 5)
    await expect(player).toHaveCount(0)
    await expect(page.getByRole('heading', { name: 'Releases', exact: true })).toBeVisible()
    await expect(page.locator('audio')).toHaveCount(1)
    expect(await page.locator('audio').evaluate((audio: HTMLAudioElement) => audio.paused)).toBe(false)
  })
}

test('skip navigation keeps the SPA route and focuses the content', async ({ page }) => {
  await mockLibrary(page)
  await page.goto(`/#/releases/${releaseId}`)
  await expect(page.getByRole('heading', { level: 1 })).toHaveText(summary.title)
  const skip = page.getByRole('link', { name: 'Skip to content' })
  await skip.focus()
  await page.keyboard.press('Enter')
  await expect(page.getByRole('main')).toBeFocused()
  expect(page.url()).toContain(`/#/releases/${releaseId}`)
})

test('unknown technical properties do not invent durations or prevent playback', async ({ page }) => {
  await mockLibrary(page)
  const unknown = { ...track, durationMs: null, originalAudioFile: { ...track.originalAudioFile, bitDepth: null, bitrate: null } }
  await page.route(`**/api/v0/releases/${releaseId}`, (route) => route.fulfill({ json: { ...release, tracks: [unknown, release.tracks[1]] } }))
  await page.route(`**/api/v0/tracks/${trackId}`, (route) => route.fulfill({ json: unknown }))
  await page.goto(`/#/releases/${releaseId}`)
  await expect(page.getByText('2 tracks', { exact: true })).toBeVisible()
  await expect(page.getByText('—', { exact: true })).toHaveCount(1)
  await expect(page.getByText('0:15', { exact: true })).toBeVisible()
  await page.getByRole('button', { name: 'Play First Light', exact: true }).click()
  const player = page.getByRole('dialog', { name: 'Player' })
  await expect(player.getByRole('slider', { name: 'Playback position' })).toBeEnabled()
  await expect(player.getByText(/ bit/)).toHaveCount(0)
  await expect.poll(() => page.locator('audio').evaluate((audio: HTMLAudioElement) => audio.currentTime)).toBeGreaterThan(0)
  await expect(page.getByRole('alert')).toHaveCount(0)
})

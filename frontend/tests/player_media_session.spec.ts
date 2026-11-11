import { test, expect } from '@playwright/test'
import { mockLibrary, releaseId } from './fixtures'

test('browser track actions follow queue availability, history and repeat mode', async ({ page }) => {
  await page.addInitScript(() => {
    const handlers: Partial<Record<MediaSessionAction, MediaSessionActionHandler | null>> = {}
    Object.assign(window, { mediaHandlers: handlers })
    const prototype = Object.getPrototypeOf(navigator.mediaSession) as MediaSession
    const setPosition = prototype.setPositionState
    prototype.setPositionState = function (state) {
      Object.assign(window, { mediaPosition: state })
      if (state) setPosition.call(this, state)
      else setPosition.call(this)
    }
    const original = prototype.setActionHandler
    prototype.setActionHandler = function (action, handler) {
      handlers[action] = handler
      original.call(this, action, handler)
    }
  })
  const availability = () => page.evaluate(() => {
    const handlers = (window as unknown as { mediaHandlers: Partial<Record<MediaSessionAction, MediaSessionActionHandler | null>> }).mediaHandlers
    return { previous: !!handlers.previoustrack, next: !!handlers.nexttrack }
  })
  const invoke = (action: MediaSessionAction) => page.evaluate((action) => {
    const handlers = (window as unknown as { mediaHandlers: Partial<Record<MediaSessionAction, MediaSessionActionHandler | null>> }).mediaHandlers
    if (!handlers[action]) throw new Error(`Missing handler: ${action}`)
    handlers[action]({ action })
  }, action)
  await mockLibrary(page)
  await page.goto(`/#/releases/${releaseId}`)
  await expect.poll(availability).toEqual({ previous: false, next: false })
  await page.getByRole('button', { name: 'Play First Light', exact: true }).click()
  const mini = page.getByRole('complementary', { name: 'Now playing' })
  await expect(mini.getByRole('button', { name: 'Pause', exact: true })).toBeVisible()
  await expect.poll(availability).toEqual({ previous: false, next: false })
  await expect.poll(() => page.evaluate(() => navigator.mediaSession.metadata?.album)).toBe('Evening Sessions')
  await expect.poll(() => page.evaluate(() => navigator.mediaSession.playbackState)).toBe('playing')
  await invoke('pause')
  await expect.poll(() => page.evaluate(() => navigator.mediaSession.playbackState)).toBe('paused')
  await expect.poll(() => page.evaluate(() => document.querySelector('audio')?.duration)).toBe(15)
  await page.evaluate(() => {
    const handlers = (window as unknown as { mediaHandlers: Partial<Record<MediaSessionAction, MediaSessionActionHandler>> }).mediaHandlers
    if (handlers.seekbackward || handlers.seekforward) throw new Error('Relative seeking must not replace OS track controls')
    handlers.seekto?.({ action: 'seekto', seekTime: 5 })
  })
  await expect.poll(() => page.evaluate(() => document.querySelector('audio')?.currentTime)).toBe(5)
  await expect.poll(() => page.evaluate(() => (window as unknown as { mediaPosition: MediaPositionState }).mediaPosition)).toEqual({ duration: 15, position: 5, playbackRate: 1 })
  await page.evaluate(() => {
    const metadata = navigator.mediaSession.metadata
    if (!metadata?.artwork[0]) throw new Error('Missing artwork')
    return new Promise<void>((resolve, reject) => {
      const image = new Image()
      image.onload = () => image.naturalWidth === 512 ? resolve() : reject(new Error('Artwork resolution'))
      image.onerror = reject
      image.src = metadata.artwork[0].src
    })
  })
  await page.getByRole('button', { name: 'Add to queue: Second Light' }).click()
  await expect.poll(availability).toEqual({ previous: false, next: true })
  await page.evaluate(() => Object.assign(window, { originalAudio: document.querySelector('audio') }))
  await invoke('nexttrack')
  await expect(mini.getByRole('link', { name: 'Open player: Second Light' })).toBeVisible()
  await expect.poll(() => page.evaluate(() => navigator.mediaSession.metadata?.title)).toBe('Second Light')
  await expect.poll(() => page.evaluate(() => document.querySelector('audio') === (window as unknown as { originalAudio: HTMLAudioElement }).originalAudio)).toBe(true)
  await expect(mini.getByRole('button', { name: 'Pause', exact: true })).toBeVisible()
  await expect.poll(availability).toEqual({ previous: true, next: false })
  await invoke('previoustrack')
  await expect(mini.getByRole('link', { name: 'Open player: First Light' })).toBeVisible()
  await expect.poll(availability).toEqual({ previous: false, next: true })
  await mini.getByRole('button', { name: 'Repeat mode' }).click()
  await expect.poll(availability).toEqual({ previous: false, next: true })
  await mini.getByRole('button', { name: 'Repeat mode' }).click()
  await expect.poll(availability).toEqual({ previous: true, next: true })
  await invoke('previoustrack')
  await expect(mini.getByRole('link', { name: 'Open player: Second Light' })).toBeVisible()
  await invoke('nexttrack')
  await expect(mini.getByRole('link', { name: 'Open player: First Light' })).toBeVisible()
  await mini.getByRole('button', { name: 'Repeat mode' }).click()
  await expect.poll(availability).toEqual({ previous: true, next: true })
  await mini.getByRole('button', { name: 'Repeat mode' }).click()
  await expect.poll(availability).toEqual({ previous: false, next: true })
  await mini.getByRole('button', { name: 'Open queue' }).click()
  await page.getByRole('button', { name: 'Clear queue', exact: true }).click()
  await expect(page.locator('audio')).toHaveCount(0)
  await expect.poll(availability).toEqual({ previous: false, next: false })
  await expect.poll(() => page.evaluate(() => navigator.mediaSession.metadata)).toBeNull()
  await expect.poll(() => page.evaluate(() => navigator.mediaSession.playbackState)).toBe('none')
  await expect.poll(() => page.evaluate(() => (window as unknown as { mediaPosition?: MediaPositionState }).mediaPosition)).toBeUndefined()
})

for (const support of ['absent', 'partial'] as const) {
  test(`playback remains usable with ${support} Media Session support`, async ({ page }) => {
    const errors: string[] = []
    page.on('pageerror', (error) => errors.push(error.message))
    await page.addInitScript((support) => {
      if (support === 'absent') {
        Reflect.deleteProperty(Object.getPrototypeOf(navigator), 'mediaSession')
      } else {
        const prototype = Object.getPrototypeOf(navigator.mediaSession) as MediaSession
        const original = prototype.setActionHandler
        prototype.setActionHandler = function (action, handler) {
          if (action === 'seekto') throw new DOMException('Unsupported action', 'NotSupportedError')
          original.call(this, action, handler)
        }
      }
    }, support)
    await mockLibrary(page)
    await page.goto(`/#/releases/${releaseId}`)
    await page.getByRole('button', { name: 'Play First Light', exact: true }).click()
    const mini = page.getByRole('complementary', { name: 'Now playing' })
    await expect(mini.getByRole('button', { name: 'Pause', exact: true })).toBeVisible()
    await page.getByRole('button', { name: 'Add to queue: Second Light' }).click()
    await mini.getByRole('button', { name: 'Next track', exact: true }).click()
    await expect(mini.getByRole('link', { name: 'Open player: Second Light' })).toBeVisible()
    await expect(mini.getByRole('button', { name: 'Pause', exact: true })).toBeVisible()
    expect(errors).toEqual([])
  })
}

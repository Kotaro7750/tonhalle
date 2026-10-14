import { test, expect } from '@playwright/test'
import { mockLibrary, releaseId } from './fixtures'

test('browser track actions follow queue availability, history and repeat mode', async ({ page }) => {
  await page.addInitScript(() => {
    const handlers: Partial<Record<MediaSessionAction, MediaSessionActionHandler | null>> = {}
    Object.assign(window, { mediaHandlers: handlers })
    const original = navigator.mediaSession.setActionHandler.bind(navigator.mediaSession)
    navigator.mediaSession.setActionHandler = (action, handler) => {
      handlers[action] = handler
      original(action, handler)
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
  await page.getByRole('button', { name: 'Add to queue: Second Light' }).click()
  await expect.poll(availability).toEqual({ previous: false, next: true })
  await invoke('nexttrack')
  await expect(mini.getByRole('link', { name: 'Open player: Second Light' })).toBeVisible()
  await expect.poll(() => page.evaluate(() => navigator.mediaSession.metadata?.title)).toBe('Second Light')
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
})

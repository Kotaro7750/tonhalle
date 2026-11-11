import { test, expect } from '@playwright/test'
import { mockLibrary, releaseId } from './fixtures'

test('installed app identity and icons resolve within the application scope', async ({ page }) => {
  await mockLibrary(page)
  await page.goto('/')
  const identity = await page.evaluate(async () => {
    const link = document.querySelector<HTMLLinkElement>('link[rel="manifest"]')!
    const response = await fetch(link.href)
    if (!response.ok) throw new Error('Manifest failed to load')
    const manifest = await response.json()
    const icons = [...manifest.icons.map((icon: { src: string; sizes: string }) => ({
      src: new URL(icon.src, link.href).href, size: Number(icon.sizes.split('x')[0]),
    })), { src: document.querySelector<HTMLLinkElement>('link[rel="apple-touch-icon"]')!.href, size: 180 }]
    const sizes = await Promise.all(icons.map(({ src, size }) => new Promise<boolean>((resolve, reject) => {
      const image = new Image()
      image.onload = () => resolve(image.naturalWidth === size && image.naturalHeight === size)
      image.onerror = reject
      image.src = src
    })))
    return {
      name: manifest.name, display: manifest.display, sizes,
      scope: new URL(manifest.scope, link.href).href,
      start: new URL(manifest.start_url, link.href).href,
      root: new URL('./', location.href).href,
    }
  })
  expect(identity.name).toBe('Tonhalle')
  expect(identity.display).toBe('standalone')
  expect(identity.scope).toBe(identity.root)
  expect(identity.start).toBe(identity.root)
  expect(identity.sizes).toEqual([true, true, true])
})

test('browser chrome follows the selected application theme', async ({ page }) => {
  await mockLibrary(page)
  await page.goto('/')
  const colorMatches = () => page.evaluate(() =>
    document.querySelector<HTMLMetaElement>('meta[name="theme-color"]')?.content === getComputedStyle(document.body).backgroundColor)
  await expect.poll(colorMatches).toBe(true)
  await page.getByRole('switch', { name: 'Dark mode' }).press('Space')
  await expect.poll(colorMatches).toBe(true)
})

test('natural endings advance the queue while seeking a paused track to its end stays paused', async ({ page }) => {
  await mockLibrary(page)
  await page.goto(`/#/releases/${releaseId}`)
  await page.getByRole('button', { name: 'Play First Light', exact: true }).click()
  await page.getByRole('button', { name: 'Add to queue: Second Light' }).click()
  const mini = page.getByRole('complementary', { name: 'Now playing' })
  const audio = page.locator('audio')
  await expect(mini.getByRole('button', { name: 'Pause', exact: true })).toBeVisible()
  await expect.poll(() => audio.evaluate((element: HTMLAudioElement) => element.duration)).toBe(15)
  await audio.evaluate((element: HTMLAudioElement) => { element.currentTime = 14.8 })
  await expect(mini.getByRole('link', { name: 'Open player: Second Light' })).toBeVisible()
  await expect(mini.getByRole('button', { name: 'Pause', exact: true })).toBeVisible()
  await mini.getByRole('button', { name: 'Pause', exact: true }).click()
  await mini.getByRole('button', { name: 'Repeat mode' }).click()
  await mini.getByRole('button', { name: 'Repeat mode' }).click()
  await mini.getByRole('button', { name: 'Forward 15 seconds' }).click()
  await expect.poll(() => audio.evaluate((element: HTMLAudioElement) => ({ time: element.currentTime, paused: element.paused }))).toEqual({ time: 15, paused: true })
  await expect(mini.getByRole('link', { name: 'Open player: Second Light' })).toBeVisible()
  await mini.getByRole('button', { name: 'Play', exact: true }).click()
  await expect.poll(() => audio.evaluate((element: HTMLAudioElement) => element.paused)).toBe(false)
  await audio.evaluate((element: HTMLAudioElement) => { element.currentTime = 14.8 })
  await expect(mini.getByRole('link', { name: 'Open player: First Light' })).toBeVisible()
  await expect(mini.getByRole('button', { name: 'Pause', exact: true })).toBeVisible()
})

import { test, expect } from '@playwright/test'
import { credits, mockLibrary, releaseId, summary, track, secondTrackId } from './fixtures'

const alice = { id: credits[0].artistId, name: 'Alice' }
const bob = { id: credits[1].artistId, name: 'Bob' }

test('remembers the library view after returning home and reloading', async ({ page }) => {
  await page.goto('/')
  await page.getByRole('tab', { name: 'Artists', exact: true }).click()
  await page.getByRole('link', { name: 'Alice', exact: true }).click()
  await page.getByRole('banner').getByRole('link', { name: 'tonhalle', exact: true }).click()
  await expect(page.getByRole('tab', { name: 'Artists', exact: true })).toHaveAttribute('aria-selected', 'true')
  await page.reload()
  await expect(page.getByRole('tab', { name: 'Artists', exact: true })).toHaveAttribute('aria-selected', 'true')
  await page.getByRole('tab', { name: 'Releases', exact: true }).click()
  await expect(page.getByRole('tab', { name: 'Releases', exact: true })).toHaveAttribute('aria-selected', 'true')
  await page.goto(`/#/releases/${releaseId}`)
  await page.getByRole('banner').getByRole('link', { name: 'tonhalle', exact: true }).click()
  await expect(page.getByRole('tab', { name: 'Releases', exact: true })).toHaveAttribute('aria-selected', 'true')
  await page.goto('/#/?view=artists')
  await expect(page.getByRole('tab', { name: 'Artists', exact: true })).toHaveAttribute('aria-selected', 'true')
})

test('artist counts omit unknown duration and handle an empty collection', async ({ page }) => {
  await page.route(`**/api/v0/releases/${releaseId}`, (route) => route.fulfill({ json: { ...summary, tracks: [{ ...track, durationMs: null }] } }))
  await page.goto(`/#/artists/${alice.id}`)
  await expect(page.getByText('1 release · 1 track', { exact: true })).toBeVisible()
  await page.route('**/api/v0/releases?*', (route) => route.fulfill({ json: { items: [] } }))
  await page.reload()
  await expect(page.getByText('0 releases · 0 tracks · 0:00', { exact: true })).toBeVisible()
})

test('artist actions include every release page and support queueing and playback', async ({ page }) => {
  await page.route('**/api/v0/releases?*', (route) => {
    const params = new URL(route.request().url()).searchParams
    expect(params.get('artistId')).toBe(alice.id)
    return route.fulfill({ json: params.has('cursor') ? { items: [{ ...summary, id: 'release-next' }] } : { items: [summary], nextCursor: 'next' } })
  })
  await page.route(`**/api/v0/releases/${releaseId}`, (route) => route.fulfill({ json: { ...summary, tracks: [track] } }))
  await page.route('**/api/v0/releases/release-next', (route) => route.fulfill({ json: { ...summary, id: 'release-next', tracks: [{ ...track, id: secondTrackId, title: 'Second Light' }] } }))
  await page.goto(`/#/artists/${alice.id}`)
  await expect(page.getByText('2 releases · 2 tracks · 0:30', { exact: true })).toBeVisible()
  await page.getByRole('button', { name: 'Add artist: Alice to queue', exact: true }).click()
  const mini = page.getByRole('complementary', { name: 'Now playing' })
  await expect(mini.getByLabel('2 tracks in queue')).toBeVisible()
  await expect(page.locator('audio')).toHaveCount(0)
  await expect(page.getByRole('button', { name: 'Queued artist: Alice', exact: true })).toBeDisabled()
  await page.getByRole('button', { name: 'Play artist: Alice', exact: true }).click()
  await expect(mini.getByRole('link', { name: 'Open player: First Light' })).toBeVisible()
  await expect(mini.getByRole('button', { name: 'Pause', exact: true })).toBeVisible()
  await mini.getByRole('button', { name: 'Open queue' }).click()
  const queue = page.getByRole('dialog', { name: 'Queue', exact: true })
  await expect(queue.locator('[data-queue-state="upcoming"]')).toHaveCount(1)
  await expect(queue.locator('[data-queue-state="upcoming"]')).toContainText('Second Light')
})

test.beforeEach(async ({ page }) => {
  await mockLibrary(page)
  await page.route('**/api/v0/artists?*', (route) => {
    const cursor = new URL(route.request().url()).searchParams.get('cursor')
    return route.fulfill({ json: cursor ? { items: [alice, bob] } : { items: [alice], nextCursor: 'next-artists' } })
  })
  await page.route('**/api/v0/artists/*', (route) => route.fulfill({ json: route.request().url().endsWith(alice.id) ? alice : bob }))
})

test('switches library view, paginates artists and opens filtered release cards', async ({ page }) => {
  const filters: (string | null)[] = []
  await page.route('**/api/v0/releases?*', (route) => {
    const params = new URL(route.request().url()).searchParams
    filters.push(params.get('artistId'))
    return route.fulfill({ json: params.has('cursor') ? { items: [{ ...summary, id: 'release-next', title: 'Next release' }] } : { items: [summary], ...(params.has('artistId') ? { nextCursor: 'next-release' } : {}) } })
  })
  await page.goto('/')
  await expect(page.getByRole('heading', { name: 'Releases', exact: true })).toBeVisible()
  await page.getByRole('tab', { name: 'Artists', exact: true }).click()
  await expect(page.getByRole('list', { name: 'Release artists' })).toBeVisible()
  await expect(page.getByRole('link', { name: 'Bob', exact: true })).toBeVisible()
  await expect(page.getByRole('link', { name: 'Alice', exact: true })).toHaveCount(1)
  await page.getByRole('link', { name: 'Alice', exact: true }).click()
  await expect(page.getByRole('heading', { level: 1 })).toHaveText('Alice')
  await expect(page.getByRole('heading', { level: 2, name: 'Releases', exact: true })).toBeVisible()
  await expect(page.getByRole('link', { name: /Evening Sessions/ })).toBeVisible()
  await page.evaluate(() => window.scrollTo(0, document.documentElement.scrollHeight))
  await expect(page.getByRole('heading', { name: 'Next release' })).toBeVisible()
  expect(filters.filter((id) => id === alice.id).length).toBeGreaterThanOrEqual(2)
  await page.reload()
  await expect(page.getByRole('heading', { level: 1 })).toHaveText('Alice')
  await page.getByRole('link', { name: 'Artists', exact: true }).click()
  await expect(page.getByRole('list', { name: 'Release artists' })).toBeVisible()
  await page.getByRole('tab', { name: 'Releases', exact: true }).click()
  await expect(page.getByRole('heading', { name: 'Releases', exact: true })).toBeVisible()
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true)
})

test('each release credit links to its artist while preserving join phrases', async ({ page }) => {
  await page.goto(`/#/releases/${releaseId}`)
  const heading = page.getByRole('heading', { level: 1 }).locator('..')
  await expect(heading).toContainText('Alice & Bob')
  await heading.getByRole('link', { name: 'Bob', exact: true }).click()
  await expect(page.getByRole('heading', { level: 1 })).toHaveText('Bob')
  expect(page.url()).toContain(`/artists/${bob.id}`)
})

test('empty artists and detail errors allow recovery', async ({ page }) => {
  await page.route('**/api/v0/artists?*', (route) => route.fulfill({ json: { items: [] } }))
  await page.goto('/#/?view=artists')
  await expect(page.getByText('No release artists yet.')).toBeVisible()
  await page.route(`**/api/v0/artists/${alice.id}`, (route) => route.fulfill({ status: 404, json: { title: 'Artist not found' } }))
  await page.goto(`/#/artists/${alice.id}`)
  await expect(page.getByRole('alert')).toContainText('Artist not found')
  await page.unroute(`**/api/v0/artists/${alice.id}`)
  await page.getByRole('button', { name: 'Retry', exact: true }).click()
  await expect(page.getByRole('heading', { level: 1 })).toHaveText('Alice')
})

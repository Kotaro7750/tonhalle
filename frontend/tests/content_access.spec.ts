import { test, expect } from '@playwright/test'
import { mockLibrary } from './fixtures'

test('content access must be configured before loading music; failure can be retried', async ({ page }) => {
  await mockLibrary(page)
  let ready = false
  let metadataRequests = 0
  await page.route('**/api/v0/releases?*', (route) => {
    metadataRequests++
    return route.fulfill({ json: { items: [] } })
  })
  await page.route('**/api/v0/content-access', (route) => ready
    ? route.fulfill({ json: {} })
    : route.fulfill({ status: 500, json: { detail: 'Could not configure content access.' } }))
  await page.goto('/')
  await expect(page.getByRole('alert')).toContainText('Could not configure content access.')
  expect(metadataRequests).toBe(0)
  ready = true
  await page.getByRole('button', { name: 'Retry', exact: true }).click()
  await expect(page.getByText('Your library is empty')).toBeVisible()
})

test('content cookies are accepted and renewed before expiry without interrupting the library', async ({ page }) => {
  await mockLibrary(page)
  let count = 0
  await page.route('**/api/v0/content-access', (route) => {
    count++
    return route.fulfill({
      headers: { 'Set-Cookie': 'content-access=test; Path=/; HttpOnly; SameSite=Lax' },
      json: { invalidAfter: new Date(Date.now() + 2000).toISOString() },
    })
  })
  await page.goto('/')
  await expect(page.getByRole('heading', { name: 'Releases', exact: true })).toBeVisible()
  await expect.poll(() => count).toBeGreaterThanOrEqual(2)
  expect((await page.context().cookies()).some((cookie) => cookie.name === 'content-access')).toBe(true)
  await expect(page.getByRole('heading', { name: 'Releases', exact: true })).toBeVisible()
})

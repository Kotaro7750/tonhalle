import { test, expect } from '@playwright/test'
import { mockLibrary, releaseId } from './fixtures'

test('player handle drags between expanded, mini and minimized states without interrupting audio', async ({ page }, testInfo) => {
  await mockLibrary(page)
  await page.goto(`/#/releases/${releaseId}`)
  await page.getByRole('button', { name: 'Play First Light', exact: true }).click()
  const mini = page.getByRole('complementary', { name: 'Now playing' })
  const dialog = page.getByRole('dialog', { name: 'Player' })
  await expect(mini.getByRole('button', { name: 'Pause', exact: true })).toBeVisible()
  const audio = await page.locator('audio').elementHandle()
  const drag = async (label: string, direction: number) => {
    const handle = page.getByRole('button', { name: label, exact: true })
    await handle.click({ trial: true })
    const bounds = (await handle.boundingBox())!
    const x = bounds.x + bounds.width / 2
    const y = bounds.y + bounds.height / 2
    if (testInfo.project.name === 'mobile') {
      const session = await page.context().newCDPSession(page)
      await session.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [{ x, y }] })
      for (let step = 1; step <= 5; step++) {
        await session.send('Input.dispatchTouchEvent', { type: 'touchMove', touchPoints: [{ x, y: y + direction * 32 * step / 5 }] })
      }
      await session.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] })
      await session.detach()
    } else {
      await page.mouse.move(x, y)
      await page.mouse.down()
      await page.mouse.move(x, y + direction * 32, { steps: 5 })
      await page.mouse.up()
    }
  }
  await drag('Expand player', -1)
  await expect(dialog).toBeVisible()
  await drag('Collapse player', 1)
  await expect(dialog).toHaveCount(0)
  await drag('Expand player', 1)
  await expect(mini.getByRole('button', { name: 'Restore mini player' })).toBeVisible()
  await expect(mini.getByRole('link')).toHaveCount(0)
  await expect(mini.getByRole('button')).toHaveCount(1)
  await drag('Restore mini player', -1)
  await expect(mini.getByRole('button', { name: 'Pause', exact: true })).toBeVisible()
  await mini.getByRole('button', { name: 'Expand player' }).press('ArrowDown')
  await mini.getByRole('button', { name: 'Restore mini player' }).press('Enter')
  await expect(mini.getByRole('button', { name: 'Expand player' })).toBeVisible()
  expect(await audio!.evaluate((element: HTMLAudioElement) => element.paused)).toBe(false)
  await expect(page.locator('audio')).toHaveCount(1)
  expect(page.url()).toContain(`/releases/${releaseId}`)
})

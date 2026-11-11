import { defineConfig, devices } from '@playwright/test'
import base from './playwright.config'

export default defineConfig({
  ...base,
  webServer: {
    command: 'npm run build && npm run preview -- --host 127.0.0.1 --port 5173 --strictPort',
    url: 'https://127.0.0.1:5173',
    ignoreHTTPSErrors: true,
    reuseExistingServer: false,
  },
  testMatch: ['player_media_session.spec.ts', 'native_app.spec.ts', 'player_controls.spec.ts'],
  projects: [
    { name: 'chrome', use: { ...devices['Desktop Chrome'], channel: 'chrome' } },
    { name: 'android-chrome', use: { ...devices['Pixel 7'], channel: 'chrome' } },
    { name: 'ios-webkit', use: { ...devices['iPhone 13'] } },
    { name: 'safari-webkit', use: { ...devices['Desktop Safari'] } },
  ],
})

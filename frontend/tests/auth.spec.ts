import { expect, test } from '@playwright/test';
import { mockLibrary } from './fixtures';

const authority = 'https://cognito-idp.us-east-1.amazonaws.com/us-east-1_test';
const storageKey = `oidc.user:${authority}:tonhalle`;

test.beforeEach(async ({ page }) => { await mockLibrary(page); });

test('a protected route starts the OAuth authorization code flow', async ({ page }) => {
  await page.route(`${authority}/.well-known/openid-configuration`, (route) => route.fulfill({ json: {
    issuer: authority,
    authorization_endpoint: 'http://identity.test/authorize',
    token_endpoint: 'http://identity.test/token',
  } }));
  const authorizeRequest = page.waitForRequest('http://identity.test/authorize?*');
  await page.route('http://identity.test/authorize?*', (route) => route.abort());
  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'Sign in to Tonhalle' })).toBeVisible();
  await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
  const url = new URL((await authorizeRequest).url());
  expect(url.searchParams.get('response_type')).toBe('code');
  expect(url.searchParams.get('client_id')).toBe('tonhalle');
  expect(url.searchParams.get('code_challenge_method')).toBe('S256');
  expect(url.searchParams.get('resource')).toBe('https://127.0.0.1:4174');
  expect(url.searchParams.get('redirect_uri')).toBe('https://127.0.0.1:4174/');
});

test('an authenticated API request includes the bearer access token', async ({ page }) => {
  await page.addInitScript(({ key }) => {
    sessionStorage.setItem(key, JSON.stringify({
      access_token: 'test-access-token', token_type: 'Bearer',
      profile: { sub: 'test-user' }, expires_at: Math.floor(Date.now() / 1000) + 3600,
      scope: 'openid profile',
    }));
  }, { key: storageKey });
  let authorization: string | undefined;
  let contentAccessAuthorization: string | undefined;
  await page.unroute('**/api/v0/content-access');
  await page.route('**/api/v0/content-access', (route) => {
    contentAccessAuthorization = route.request().headers().authorization;
    return route.fulfill({ json: {} });
  });
  await page.route('**/api/v0/releases?*', (route) => {
    authorization = route.request().headers().authorization;
    return route.fulfill({ json: { items: [] } });
  });
  await page.goto('/');
  await expect(page.getByText('Your library is empty')).toBeVisible();
  expect(authorization).toBe('Bearer test-access-token');
  expect(contentAccessAuthorization).toBe('Bearer test-access-token');
  await expect(page.getByRole('button', { name: 'Sign out' })).toBeVisible();
});

test('Cognito logout includes its required client and logout URI', async ({ page }) => {
  await page.addInitScript(({ key }) => {
    sessionStorage.setItem(key, JSON.stringify({
      access_token: 'test-access-token', id_token: 'test-id-token', token_type: 'Bearer',
      profile: { sub: 'test-user' }, expires_at: Math.floor(Date.now() / 1000) + 3600,
      scope: 'openid profile',
    }));
  }, { key: storageKey });
  await page.route(`${authority}/.well-known/openid-configuration`, (route) => route.fulfill({ json: {
    issuer: authority,
    authorization_endpoint: 'http://identity.test/authorize',
    token_endpoint: 'http://identity.test/token',
    end_session_endpoint: 'http://identity.test/logout',
  } }));
  const logoutRequest = page.waitForRequest('http://identity.test/logout?*');
  const clearRequest = page.waitForRequest((request) => request.url().endsWith('/api/v0/content-access') && request.method() === 'DELETE');
  await page.route('http://identity.test/logout?*', (route) => route.abort());
  await page.goto('/');
  await page.getByRole('button', { name: 'Sign out' }).click();
  const clear = await clearRequest;
  expect(clear.headers().authorization).toBe('Bearer test-access-token');
  const url = new URL((await logoutRequest).url());
  expect(url.searchParams.get('client_id')).toBe('tonhalle');
  expect(url.searchParams.get('logout_uri')).toBe('https://127.0.0.1:4174/');
});

test('content access is refreshed before invalidAfter', async ({ page }) => {
  await page.addInitScript(({ key }) => {
    sessionStorage.setItem(key, JSON.stringify({
      access_token: 'test-access-token', token_type: 'Bearer',
      profile: { sub: 'test-user' }, expires_at: Math.floor(Date.now() / 1000) + 3600,
      scope: 'openid profile',
    }));
  }, { key: storageKey });
  await page.unroute('**/api/v0/content-access');
  let configureCount = 0;
  await page.route('**/api/v0/content-access', (route) => {
    configureCount++;
    return route.fulfill({ json: { invalidAfter: new Date(Date.now() + 1_000).toISOString() } });
  });
  await page.goto('/');
  await expect.poll(() => configureCount).toBeGreaterThanOrEqual(2);
});

test('failed content access removal keeps the session and allows another sign out attempt', async ({ page }) => {
  await page.addInitScript(({ key }) => {
    sessionStorage.setItem(key, JSON.stringify({
      access_token: 'test-access-token', token_type: 'Bearer',
      profile: { sub: 'test-user' }, expires_at: Math.floor(Date.now() / 1000) + 3600,
      scope: 'openid',
    }));
  }, { key: storageKey });
  let clears = 0;
  await page.route('**/api/v0/content-access', (route) => {
    if (route.request().method() === 'DELETE') {
      clears++;
      return route.fulfill({ status: 500, json: {} });
    }
    return route.fulfill({ json: {} });
  });
  await page.goto('/');
  await page.getByRole('button', { name: 'Sign out' }).click();
  await expect(page.getByRole('alert')).toContainText('Could not sign out.');
  expect(await page.evaluate((key) => sessionStorage.getItem(key), storageKey)).toContain('test-access-token');
  await page.getByRole('button', { name: 'Sign out' }).click();
  await expect.poll(() => clears).toBe(2);
});

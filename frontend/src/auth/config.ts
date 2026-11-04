import { UserManager, type User } from 'oidc-client-ts'

export const authEnabled = import.meta.env.VITE_AUTH_ENABLED === 'true'

function required(name: string, value: string | undefined) {
  if (!value) throw new Error(`認証を有効にするには ${name} が必要です。`)
  return value
}

// Use the SPA deployment root, not the route at which authentication starts.
// This URL must be registered with the identity provider.
const appUrl = new URL(import.meta.env.BASE_URL, window.location.origin).href
const authority = authEnabled
  ? required('VITE_AUTH_AUTHORITY', import.meta.env.VITE_AUTH_AUTHORITY)
  : undefined
const clientId = authEnabled
  ? required('VITE_AUTH_CLIENT_ID', import.meta.env.VITE_AUTH_CLIENT_ID)
  : undefined
const resource = import.meta.env.VITE_AUTH_RESOURCE || window.location.origin
const postLogoutRedirectUri = appUrl

export const userManager = authEnabled ? new UserManager({
  authority: authority!,
  client_id: clientId!,
  redirect_uri: appUrl,
  post_logout_redirect_uri: postLogoutRedirectUri,
  response_type: 'code',
  scope: import.meta.env.VITE_AUTH_SCOPE || 'openid',
  resource,
  automaticSilentRenew: true,
}) : undefined

export const signoutRedirectArgs = authority && new URL(authority).hostname.startsWith('cognito-idp.')
  ? { extraQueryParams: { client_id: clientId!, logout_uri: postLogoutRedirectUri } }
  : undefined

export async function getAccessToken() {
  if (!userManager) return undefined
  const user = await userManager.getUser()
  return user && !user.expired ? user.access_token : undefined
}

export function restoreRoute(user: User | undefined) {
  const state = user?.state
  const returnTo = state && typeof state === 'object' && 'returnTo' in state
    && typeof state.returnTo === 'string' ? state.returnTo : '/'
  window.location.hash = `#${returnTo}`
  window.history.replaceState({}, document.title, `${window.location.pathname}${window.location.hash}`)
}

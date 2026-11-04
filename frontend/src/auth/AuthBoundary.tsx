import type { PropsWithChildren } from 'react'
import { AuthProvider } from 'react-oidc-context'
import { authEnabled, restoreRoute, userManager } from './config'

export function AuthBoundary({ children }: PropsWithChildren) {
  if (!authEnabled || !userManager) return children
  return <AuthProvider userManager={userManager} onSigninCallback={restoreRoute}>
    {children}
  </AuthProvider>
}

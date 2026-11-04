import { Button, Heading, Stack, Text } from '@chakra-ui/react'
import { useState, type PropsWithChildren } from 'react'
import { useAuth } from 'react-oidc-context'
import { useLocation } from 'react-router-dom'
import { Loading } from '../components/Feedback'
import { authEnabled } from './config'

function AuthenticationRequired({ children }: PropsWithChildren) {
  const auth = useAuth()
  const location = useLocation()
  const [error, setError] = useState(false)
  if (auth.isLoading || auth.activeNavigator) return <Loading />
  if (auth.isAuthenticated) return children
  return <Stack align="center" py="20" px={{ base: '5', md: '8' }} gap="5">
    <Heading size="xl">Sign in to Tonhalle</Heading>
    {(auth.error || error) && <Text role="alert" color="fg.error">Sign in failed. Please try again.</Text>}
    <Button onClick={() => {
      setError(false)
      void auth.signinRedirect({ state: { returnTo: `${location.pathname}${location.search}` } }).catch(() => setError(true))
    }}>Sign in</Button>
  </Stack>
}

export function ProtectedRoute({ children }: PropsWithChildren) {
  return authEnabled ? <AuthenticationRequired>{children}</AuthenticationRequired> : children
}

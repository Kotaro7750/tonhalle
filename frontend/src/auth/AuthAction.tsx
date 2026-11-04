import { HStack, IconButton, Text } from '@chakra-ui/react'
import { useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import { useAuth } from 'react-oidc-context'
import { LogOut } from '../components/Icons'
import { clearContentAccess } from '../content-access/api'
import { usePlayer } from '../player/store'
import { useAudioControls } from '../player/PlayerProvider'
import { authEnabled, signoutRedirectArgs } from './config'

function EnabledAuthAction() {
  const auth = useAuth()
  const queryClient = useQueryClient()
  const { toggle } = useAudioControls()
  const [signingOut, setSigningOut] = useState(false)
  const [error, setError] = useState(false)
  if (!auth.isAuthenticated) return null
  return <HStack>
    {error && <Text role="alert" color="fg.error">Could not sign out.</Text>}
    <IconButton aria-label="Sign out" title="ログアウト" variant="ghost" loading={signingOut} onClick={() => {
      setSigningOut(true)
      setError(false)
      if (usePlayer.getState().playing) toggle()
      void clearContentAccess().then(async () => {
        await queryClient.cancelQueries()
        queryClient.clear()
        usePlayer.setState(usePlayer.getInitialState())
        await auth.signoutRedirect(signoutRedirectArgs)
      }).catch(() => { setError(true); setSigningOut(false) })
    }}><LogOut /></IconButton>
  </HStack>
}

export function AuthAction() {
  return authEnabled ? <EnabledAuthAction /> : null
}

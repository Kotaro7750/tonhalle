import { Alert, IconButton, Center, Heading, Spinner, Stack, Text } from '@chakra-ui/react'
import { RefreshCw } from './Icons'
import { ApiError } from '../api/client'

export function Loading() {
  return <Center py="20" role="status" aria-label="Loading" gap="3"><Spinner size="sm" /></Center>
}

export function ErrorNotice({ error, retry }: { error: Error; retry: () => void }) {
  return (
    <Alert.Root role="alert" status="error" variant="subtle">
      <Alert.Indicator />
      <Alert.Content>
        <Alert.Title>{error instanceof ApiError && error.status === 404 ? 'Not found' : 'Failed to load'}</Alert.Title>
        <Alert.Description>{error instanceof ApiError ? error.message : 'Check your connection and try again.'}</Alert.Description>
        <IconButton aria-label="Retry" title="再試行" onClick={retry} variant="outline" size="sm" mt="3"><RefreshCw size={16} /></IconButton>
      </Alert.Content>
    </Alert.Root>
  )
}

export function EmptyLibrary() {
  return <Stack py="20" textAlign="center" gap="3"><Heading size="xl" as="h2">Your library is empty</Heading><Text color="fg.muted">Releases will appear here when music is added.</Text></Stack>
}

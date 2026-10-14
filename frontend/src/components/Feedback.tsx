import { Alert, Button, Center, Heading, Spinner, Stack, Text } from '@chakra-ui/react'
import { RefreshCw } from 'lucide-react'
import { ApiError } from '../api/client'

export function Loading() {
  return <Center py="20" role="status" gap="3"><Spinner size="sm" /><Text>読み込み中…</Text></Center>
}

export function ErrorNotice({ error, retry }: { error: Error; retry: () => void }) {
  return (
    <Alert.Root role="alert" status="error" variant="subtle">
      <Alert.Indicator />
      <Alert.Content>
        <Alert.Title>{error instanceof ApiError && error.status === 404 ? '見つかりませんでした' : '読み込みに失敗しました'}</Alert.Title>
        <Alert.Description>{error instanceof ApiError ? error.message : '通信状況を確認して、もう一度お試しください。'}</Alert.Description>
        <Button onClick={retry} variant="outline" size="sm" mt="3"><RefreshCw size={16} />再試行</Button>
      </Alert.Content>
    </Alert.Root>
  )
}

export function EmptyLibrary() {
  return <Stack py="20" textAlign="center" gap="3"><Heading size="xl" as="h2">ライブラリは空です</Heading><Text color="fg.muted">音楽が追加されると、ここにリリースが表示されます。</Text></Stack>
}

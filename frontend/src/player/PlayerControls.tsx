import { Alert, Button, HStack, IconButton, Slider, Stack, Text } from '@chakra-ui/react'
import { Pause, Play, RefreshCw } from 'lucide-react'
import { formatTime } from '../format'
import { useAudioControls } from './PlayerProvider'
import { usePlayer } from './store'

export function PlayButton({ large = false }: { large?: boolean }) {
  const playing = usePlayer((state) => state.playing)
  const error = usePlayer((state) => state.error)
  const { toggle } = useAudioControls()
  return (
    <IconButton aria-label={playing ? '一時停止' : '再生'} rounded="full" size={large ? '2xl' : 'md'} onClick={toggle} disabled={!!error}>
      {playing ? <Pause /> : <Play />}
    </IconButton>
  )
}

export function PlayerControls() {
  const { currentTime, duration, ready, error } = usePlayer()
  const { seek, reload } = useAudioControls()
  return (
    <Stack gap="6" width="full">
      <Slider.Root aria-label={['再生位置']} value={[currentTime]} max={duration || 1} step={0.1} disabled={!ready || !duration} onValueChange={({ value }) => seek(value[0])}>
        <Slider.Control>
          <Slider.Track><Slider.Range /></Slider.Track>
          <Slider.Thumb index={0}><Slider.HiddenInput /></Slider.Thumb>
        </Slider.Control>
      </Slider.Root>
      <HStack justify="space-between" color="fg.muted" textStyle="sm" fontVariantNumeric="tabular-nums">
        <Text>{formatTime(currentTime)}</Text><Text>{formatTime(duration)}</Text>
      </HStack>
      <HStack justify="center"><PlayButton large /></HStack>
      {error && <Alert.Root role="alert" status="error">
        <Alert.Indicator /><Alert.Content><Alert.Title>再生できません</Alert.Title><Alert.Description>{error}</Alert.Description>
          <Button mt="3" size="sm" variant="outline" onClick={reload}><RefreshCw size={16} />音源を再読み込み</Button>
        </Alert.Content>
      </Alert.Root>}
    </Stack>
  )
}

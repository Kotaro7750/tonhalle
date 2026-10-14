import { Alert, HStack, IconButton, Slider, Stack, Text } from '@chakra-ui/react'
import { Pause, Play, RefreshCw } from 'lucide-react'
import { formatTime } from '../format'
import { useAudioControls } from './PlayerProvider'
import { usePlayer } from './store'

export function PlayButton({ large = false }: { large?: boolean }) {
  const playing = usePlayer((state) => state.playing)
  const error = usePlayer((state) => state.error)
  const { toggle } = useAudioControls()
  return (
    <IconButton aria-label={playing ? 'Pause' : 'Play'} rounded="full" size={large ? '2xl' : 'md'} onClick={toggle} disabled={!!error}>
      {playing ? <Pause /> : <Play />}
    </IconButton>
  )
}

export function PlayerControls() {
  const { currentTime, duration, ready, error } = usePlayer()
  const { seek, reload } = useAudioControls()
  return (
    <Stack gap="6" width="full">
      <Slider.Root aria-label={['Playback position']} value={[currentTime]} max={duration || 1} step={0.1} disabled={!ready || !duration} onValueChange={({ value }) => seek(value[0])}>
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
        <Alert.Indicator /><Alert.Content><Alert.Title>Playback failed</Alert.Title><Alert.Description>{error}</Alert.Description>
          <IconButton aria-label="Reload audio" title="音源を再読み込み" mt="3" size="sm" variant="outline" onClick={reload}><RefreshCw size={16} /></IconButton>
        </Alert.Content>
      </Alert.Root>}
    </Stack>
  )
}

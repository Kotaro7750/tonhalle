import { Alert, Box, HStack, IconButton, Slider, Stack, Text } from '@chakra-ui/react'
import { Pause, Play, RefreshCw, Repeat1, RepeatOff, RotateCcw, RotateCw } from 'lucide-react'
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

export function PlaybackButtons({ large = false }: { large?: boolean }) {
  const { ready, duration, error, repeat, update } = usePlayer()
  const { seek, skip } = useAudioControls()
  const disabled = !ready || !duration || !!error
  return (
    <HStack gap={{ base: '2', md: '3' }} justify="center" flexShrink="0">
      <IconButton aria-label="Restart track" title="曲の最初に戻る" variant="ghost" rounded="full" disabled={disabled} onClick={() => seek(0)}><RotateCcw /></IconButton>
      <IconButton aria-label="Back 15 seconds" title="15秒戻す" variant="ghost" rounded="full" disabled={disabled} onClick={() => skip(-15)}>
        <Box position="relative" display="inline-flex"><RotateCcw /><Text as="span" position="absolute" inset="0" display="flex" alignItems="center" justifyContent="center" fontSize="2xs" fontWeight="bold">15</Text></Box>
      </IconButton>
      <PlayButton large={large} />
      <IconButton aria-label="Forward 15 seconds" title="15秒送る" variant="ghost" rounded="full" disabled={disabled} onClick={() => skip(15)}>
        <Box position="relative" display="inline-flex"><RotateCw /><Text as="span" position="absolute" inset="0" display="flex" alignItems="center" justifyContent="center" fontSize="2xs" fontWeight="bold">15</Text></Box>
      </IconButton>
      <IconButton aria-label="Repeat track" title="曲のリピート" aria-pressed={repeat} variant={repeat ? 'subtle' : 'ghost'} color={repeat ? 'fg' : 'fg.muted'} rounded="full" onClick={() => update({ repeat: !repeat })}>{repeat ? <Repeat1 /> : <RepeatOff />}</IconButton>
    </HStack>
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
      <PlaybackButtons large />
      {error && <Alert.Root role="alert" status="error">
        <Alert.Indicator /><Alert.Content><Alert.Title>Playback failed</Alert.Title><Alert.Description>{error}</Alert.Description>
          <IconButton aria-label="Reload audio" title="音源を再読み込み" mt="3" size="sm" variant="outline" onClick={reload}><RefreshCw size={16} /></IconButton>
        </Alert.Content>
      </Alert.Root>}
    </Stack>
  )
}

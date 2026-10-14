import { Alert, HStack, IconButton, Slider, Stack, Text } from '@chakra-ui/react'
import { Pause, Play, Repeat, SkipBack, SkipForward, RefreshCw, Repeat1, RepeatOff, RotateCcw, Forward15, Replay15, ShuffleRepeat } from '../components/Icons'
import { formatTime } from '../format'
import { useAudioControls } from './PlayerProvider'
import { usePlayer } from './store'
import { QueueButton } from './QueueOverlay'

export function PlayButton() {
  const track = usePlayer((state) => state.track)
  const playing = usePlayer((state) => state.playing)
  const error = usePlayer((state) => state.error)
  const { toggle } = useAudioControls()
  return (
    <IconButton aria-label={playing ? 'Pause' : 'Play'} variant="ghost" rounded="full" size="md" onClick={toggle} disabled={!track || !!error}>
      {playing ? <Pause /> : <Play />}
    </IconButton>
  )
}

export function PlaybackButtons() {
  const { track, history, queue, ready, duration, error, repeat } = usePlayer()
  const { seek, skip, previous, next, cycleRepeat } = useAudioControls()
  const disabled = !ready || !duration || !!error
  return (
    <Stack direction="row" wrap="wrap-reverse" maxW="full" justify="center" gap="2" align="center" alignSelf="center">
      <HStack role="group" aria-label="Playback" gap={{ base: '0', md: '2' }} justify="center" flexShrink="0" bg="bg.emphasized" rounded="full" p="1">
        <IconButton aria-label="Previous track" title="前の曲" variant="ghost" rounded="full" disabled={!track || (!history.length && repeat !== 'queue' && repeat !== 'shuffle')} onClick={previous}><SkipBack /></IconButton>
        <IconButton aria-label="Back 15 seconds" title="15秒戻す" variant="ghost" rounded="full" disabled={disabled} onClick={() => skip(-15)}>
          <Replay15 />
        </IconButton>
        <PlayButton />
        <IconButton aria-label="Forward 15 seconds" title="15秒送る" variant="ghost" rounded="full" disabled={disabled} onClick={() => skip(15)}>
          <Forward15 />
        </IconButton>
        <IconButton aria-label="Next track" title="次の曲" variant="ghost" rounded="full" disabled={!track || (!queue.length && repeat !== 'queue' && repeat !== 'shuffle')} onClick={next}><SkipForward /></IconButton>
      </HStack>
      <HStack gap="2">
        <HStack role="group" aria-label="Repeat and restart" gap={{ base: '0', md: '2' }} bg="bg.emphasized" rounded="full" p="1">
          <IconButton aria-label="Restart track" title="曲の最初に戻る" variant="ghost" rounded="full" disabled={disabled} onClick={() => seek(0)}><RotateCcw /></IconButton>
          <IconButton aria-label="Repeat mode" aria-description={repeat === 'off' ? 'Repeat off' : repeat === 'track' ? 'Repeat current track' : repeat === 'queue' ? 'Repeat entire queue' : 'Shuffle repeat'} disabled={!track} title={repeat === 'off' ? 'リピート: オフ（クリックで1曲）' : repeat === 'track' ? 'リピート: 1曲（クリックでキュー全体）' : repeat === 'queue' ? 'リピート: キュー全体（クリックでシャッフル）' : 'リピート: シャッフル（クリックでオフ）'} aria-pressed={repeat !== 'off'} variant={repeat === 'off' ? 'ghost' : 'subtle'} color={repeat === 'off' ? 'fg.muted' : undefined} _hover={{ bg: 'colorPalette.subtle' }} _dark={repeat !== 'off' ? { bg: 'colorPalette.solid', color: 'colorPalette.contrast', _hover: { bg: 'colorPalette.subtle', color: 'colorPalette.fg' } } : undefined} rounded="full" onClick={cycleRepeat}>{repeat === 'track' ? <Repeat1 /> : repeat === 'queue' ? <Repeat /> : repeat === 'shuffle' ? <ShuffleRepeat /> : <RepeatOff />}</IconButton>
        </HStack>
        <QueueButton />
      </HStack>
    </Stack>
  )
}

export function PlayerControls() {
  const { currentTime, duration, ready, error } = usePlayer()
  const { seek, reload } = useAudioControls()
  return (
    <Stack gap="3" width="full" mt="auto" flexShrink="0">
      <Stack gap="1">
        <Slider.Root aria-label={['Playback position']} value={[currentTime]} max={duration || 1} step={0.1} disabled={!ready || !duration} onValueChange={({ value }) => seek(value[0])}>
          <Slider.Control>
            <Slider.Track><Slider.Range /></Slider.Track>
            <Slider.Thumb index={0}><Slider.HiddenInput /></Slider.Thumb>
          </Slider.Control>
        </Slider.Root>
        <HStack justify="space-between" color="fg.muted" textStyle="sm" fontVariantNumeric="tabular-nums">
          <Text>{formatTime(currentTime)}</Text><Text>{formatTime(duration)}</Text>
        </HStack>
      </Stack>
      {error && <Alert.Root role="alert" status="error">
        <Alert.Indicator /><Alert.Content><Alert.Title>Playback failed</Alert.Title><Alert.Description>{error}</Alert.Description>
          <IconButton aria-label="Reload audio" title="音源を再読み込み" mt="3" size="sm" variant="ghost" onClick={reload}><RefreshCw size={16} /></IconButton>
        </Alert.Content>
      </Alert.Root>}
      <PlaybackButtons />
    </Stack>
  )
}

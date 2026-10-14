import { Box, Container, HStack, IconButton, LinkBox, LinkOverlay, Stack, Text } from '@chakra-ui/react'
import { Disc3, Play, Trash2 } from '../components/Icons'
import { Link as RouterLink, matchPath, useLocation, useNavigate } from 'react-router-dom'
import { useState } from 'react'
import { PlayerHandle } from './PlayerHandle'
import { artistName } from '../format'
import { usePlayer } from './store'
import { ShuffleQueueButton } from './ShuffleQueueButton'
import { QueueButton } from './QueueOverlay'
import { useAudioControls } from './PlayerProvider'
import { PlaybackButtons } from './PlayerControls'

export function MiniPlayer() {
  const track = usePlayer((state) => state.track)
  const queueLength = usePlayer((state) => state.history.length + state.queue.length)
  const update = usePlayer((state) => state.update)
  const error = usePlayer((state) => state.error)
  const playing = usePlayer((state) => state.playing)
  const { playQueue, clearQueue } = useAudioControls()
  const location = useLocation()
  const navigate = useNavigate()
  const [minimized, setMinimized] = useState(false)
  if (!track && !queueLength) return null
  const expand = () => {
    if (minimized) setMinimized(false)
    else if (!track) update({ queueOpen: true })
    else void navigate(`/tracks/${track.id}`, { state: { backgroundLocation: location } })
  }
  return (
    <Box as="aside" aria-label="Now playing" hidden={!!matchPath('/tracks/:trackId', location.pathname)} position="fixed" bottom="0" insetX="0" px={{ base: '3', md: '6' }} pb={{ base: '3', md: '4' }} zIndex="sticky" pointerEvents="none">
      <PlayerHandle minimized={minimized} label={minimized ? 'Restore mini player' : 'Expand player'} onActivate={expand} onUp={expand} onDown={() => setMinimized(true)} />
      <Box hidden={minimized} pointerEvents="auto">
        <Container asChild maxW="6xl" w={track ? 'full' : 'fit-content'} py="4" px={track ? { base: '4', md: '8' } : '4'}>
          <LinkBox bg="bg.muted" borderWidth="1" borderColor="border.emphasized" rounded="xl" shadow="md" cursor={track ? 'pointer' : 'default'} transitionProperty="box-shadow, border-color" transitionDuration="fast" _hover={{ shadow: 'lg', borderColor: 'fg.muted' }} _motionReduce={{ transition: 'none' }}>
            {track ? <Stack direction={{ base: 'column', md: 'row' }} gap="4" align={{ base: 'stretch', md: 'center' }}>
              <HStack gap="4" flex="1" minW="0">
                <Box
                  aria-hidden="true"
                  flexShrink="0"
                  animationName="spin"
                  animationDuration="4s"
                  animationTimingFunction="linear"
                  animationIterationCount="infinite"
                  animationPlayState={playing ? 'running' : 'paused'}
                  _motionReduce={{ animation: 'none' }}
                >
                  <Disc3 />
                </Box>
                <Stack gap="1" flex="1" minW="0">
                  <LinkOverlay asChild fontWeight="semibold" truncate focusRing="none" focusVisibleRing="none" data-player-link aria-label={`Open player: ${track.title}`}><RouterLink to={`/tracks/${track.id}`} state={{ backgroundLocation: location }}>{track.title}</RouterLink></LinkOverlay>
                  <Text textStyle="xs" color={error ? 'fg.error' : 'fg.muted'} truncate>{error ? 'Could not play audio. Open the player to retry.' : artistName(track.artistCredits)}</Text>
                </Stack>
              </HStack>
              <HStack position="relative" zIndex="1" justify="center"><PlaybackButtons /></HStack>
            </Stack> : <HStack justify="center" gap="4">
              <IconButton aria-label="Play queue from start" title="キューの先頭から再生" variant="ghost" rounded="full" onClick={playQueue}><Play size={18} /></IconButton>
              <ShuffleQueueButton />
              <IconButton aria-label="Clear queue" title="キューをクリア" variant="ghost" rounded="full" onClick={clearQueue}><Trash2 size={18} /></IconButton>
              <QueueButton />
            </HStack>}
          </LinkBox>
        </Container>
      </Box>
    </Box>
  )
}

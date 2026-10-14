import { Box, Container, HStack, LinkBox, LinkOverlay, Stack, Text } from '@chakra-ui/react'
import { Disc3 } from 'lucide-react'
import { Link as RouterLink, matchPath, useLocation, useNavigate } from 'react-router-dom'
import { useState } from 'react'
import { PlayerHandle } from './PlayerHandle'
import { artistName } from '../format'
import { usePlayer } from './store'
import { PlaybackButtons } from './PlayerControls'

export function MiniPlayer() {
  const track = usePlayer((state) => state.track)
  const error = usePlayer((state) => state.error)
  const playing = usePlayer((state) => state.playing)
  const location = useLocation()
  const navigate = useNavigate()
  const [minimized, setMinimized] = useState(false)
  if (!track) return null
  const expand = () => {
    if (minimized) setMinimized(false)
    else void navigate(`/tracks/${track.id}`, { state: { backgroundLocation: location } })
  }
  return (
    <Box as="aside" aria-label="Now playing" hidden={!!matchPath('/tracks/:trackId', location.pathname)} position="fixed" bottom="0" insetX="0" px={{ base: '3', md: '6' }} pb={{ base: '3', md: '4' }} zIndex="sticky" pointerEvents="none">
      <PlayerHandle minimized={minimized} label={minimized ? 'Restore mini player' : 'Expand player'} onActivate={expand} onUp={expand} onDown={() => setMinimized(true)} />
      <Box hidden={minimized} pointerEvents="auto">
        <Container asChild maxW="6xl" py="4" px={{ base: '4', md: '8' }}>
          <LinkBox bg="bg.muted" borderWidth="1" borderColor="border.emphasized" rounded="xl" shadow="md" cursor="pointer" transitionProperty="background" transitionDuration="fast" _hover={{ bg: 'bg.emphasized' }}>
            <Stack direction={{ base: 'column', md: 'row' }} gap="4" align={{ base: 'stretch', md: 'center' }}>
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
              <Box position="relative" zIndex="1"><PlaybackButtons /></Box>
            </Stack>
          </LinkBox>
        </Container>
      </Box>
    </Box>
  )
}

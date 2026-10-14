import { Box, Container, HStack, LinkBox, LinkOverlay, Stack, Text } from '@chakra-ui/react'
import { Disc3 } from 'lucide-react'
import { Link as RouterLink, matchPath, useLocation } from 'react-router-dom'
import { artistName } from '../format'
import { usePlayer } from './store'
import { PlayButton } from './PlayerControls'

export function MiniPlayer() {
  const track = usePlayer((state) => state.track)
  const error = usePlayer((state) => state.error)
  const location = useLocation()
  if (!track) return null
  return (
    <Box as="aside" aria-label="Now playing" hidden={!!matchPath('/tracks/:trackId', location.pathname)} position="fixed" bottom="0" insetX="0" px={{ base: '3', md: '6' }} pb={{ base: '3', md: '4' }} zIndex="sticky">
      <Container asChild maxW="6xl" py="4" px={{ base: '4', md: '8' }}>
        <LinkBox bg="bg.muted" borderWidth="1" borderColor="border.emphasized" rounded="xl" shadow="md" cursor="pointer" transitionProperty="background" transitionDuration="fast" _hover={{ bg: 'bg.emphasized' }}>
          <HStack gap="4" justify="space-between">
            <Disc3 aria-hidden="true" />
            <Stack gap="1" flex="1" minW="0">
              <LinkOverlay asChild fontWeight="semibold" truncate focusRing="none" focusVisibleRing="none" data-player-link aria-label={`Open player: ${track.title}`}><RouterLink to={`/tracks/${track.id}`} state={{ backgroundLocation: location }}>{track.title}</RouterLink></LinkOverlay>
              <Text textStyle="xs" color={error ? 'fg.error' : 'fg.muted'} truncate>{error ? 'Could not play audio. Open the player to retry.' : artistName(track.artistCredits)}</Text>
            </Stack>
            <Box position="relative" zIndex="1"><PlayButton /></Box>
          </HStack>
        </LinkBox>
      </Container>
    </Box>
  )
}

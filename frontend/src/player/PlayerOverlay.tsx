import { Box, CloseButton, Drawer, Portal, VisuallyHidden } from '@chakra-ui/react'
import { useEffect, useState } from 'react'
import { Route, Routes, type Location } from 'react-router-dom'
import { TrackPage } from '../pages/TrackPage'

export function PlayerOverlay({ open, location, onClose }: { open: boolean; location: Location; onClose: () => void }) {
  const [lastLocation, setLastLocation] = useState(location)
  useEffect(() => { if (open) setLastLocation(location) }, [open, location])

  return (
    <Drawer.Root
      open={open}
      onOpenChange={({ open }) => { if (!open) onClose() }}
      onFocusOutside={(event) => event.preventDefault()}
      placement="bottom"
      lazyMount
      unmountOnExit
      finalFocusEl={() => document.querySelector<HTMLElement>('[data-player-link]') ?? document.querySelector<HTMLElement>('main')}
    >
      <Portal>
        <Drawer.Backdrop _motionReduce={{ animation: 'none' }} />
        <Drawer.Positioner>
          <Drawer.Content maxW="6xl" mx="auto" maxH="92dvh" roundedTop="2xl" overflow="hidden" _motionReduce={{ animation: 'none' }}>
            <Drawer.Title asChild><VisuallyHidden>Player</VisuallyHidden></Drawer.Title>
            <Box aria-hidden="true" w="12" h="1" rounded="full" bg="border.emphasized" mx="auto" mt="3" mb="4" flexShrink="0" />
            <Drawer.CloseTrigger asChild><CloseButton aria-label="Close player" /></Drawer.CloseTrigger>
            <Drawer.Body px={{ base: '5', md: '8' }} pt="4" pb="8" minH="0">
              <Routes location={open ? location : lastLocation}>
                <Route path="/tracks/:trackId" element={<TrackPage />} />
              </Routes>
            </Drawer.Body>
          </Drawer.Content>
        </Drawer.Positioner>
      </Portal>
    </Drawer.Root>
  )
}

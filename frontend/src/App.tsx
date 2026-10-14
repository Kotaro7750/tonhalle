import { Box, Container, HStack, Link, SkipNavContent, SkipNavLink, Text } from '@chakra-ui/react'
import { ArrowLeft } from 'lucide-react'
import { Route, Routes, Link as RouterLink, matchPath, useLocation, useNavigate, type Location } from 'react-router-dom'
import { useEffect, useRef } from 'react'
import { ReleasesPage } from './pages/ReleasesPage'
import { ReleasePage } from './pages/ReleasePage'
import { MiniPlayer } from './player/MiniPlayer'
import { PlayerOverlay } from './player/PlayerOverlay'
import { ColorModeSwitch } from './components/ui/color-mode-switch'
import { Brand } from './components/Brand'

export function App() {
  const location = useLocation()
  const navigate = useNavigate()
  const playerOpen = !!matchPath('/tracks/:trackId', location.pathname)
  const backgroundLocation = location.state?.backgroundLocation as Location | undefined
  const pageLocation = playerOpen ? backgroundLocation ?? '/' : location
  const pagePathname = typeof pageLocation === 'string' ? pageLocation : pageLocation.pathname
  const main = useRef<HTMLDivElement>(null)
  useEffect(() => {
    window.scrollTo(0, 0)
    document.title = 'Tonhalle — Music Library'
  }, [pagePathname])
  useEffect(() => {
    if (!playerOpen) main.current?.focus({ preventScroll: true })
  }, [pagePathname, playerOpen])
  return (
    <>
      <SkipNavLink onClick={(event) => { event.preventDefault(); main.current?.focus() }}>Skip to content</SkipNavLink>
      <Box as="header" borderBottomWidth="1" bg="bg.panel">
        <Container maxW="6xl" px={{ base: '5', md: '8' }} py="5">
          <HStack justify="space-between">
            <Link asChild color="fg" textDecoration="none"><RouterLink to="/"><Brand /></RouterLink></Link>
            <ColorModeSwitch />
          </HStack>
        </Container>
      </Box>
      <Container as="main" ref={main} tabIndex={-1} outline="none" maxW="6xl" px={{ base: '5', md: '8' }} pt={{ base: '8', md: '12' }} pb="36">
        <SkipNavContent />
        <Routes location={pageLocation}>
          <Route path="/" element={<ReleasesPage />} />
          <Route path="/releases/:releaseId" element={<ReleasePage />} />
          <Route path="*" element={<Box><Text mb="4">Page not found.</Text><Link asChild><RouterLink to="/" aria-label="Back to library" title="ライブラリに戻る"><ArrowLeft size={16} /></RouterLink></Link></Box>} />
        </Routes>
      </Container>
      <MiniPlayer />
      <PlayerOverlay open={playerOpen} location={location} onClose={() => {
        if (backgroundLocation) void navigate(-1)
        else void navigate('/', { replace: true })
      }} />
    </>
  )
}

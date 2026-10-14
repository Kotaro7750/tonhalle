import { Box, HStack, Stack, Text } from '@chakra-ui/react'
import { useQuery } from '@tanstack/react-query'
import { useEffect } from 'react'
import { useLocation, useNavigate, useParams } from 'react-router-dom'
import { trackOptions } from '../api/queries'
import { AudioFormatBadge } from '../components/AudioFormatBadge'
import { ErrorNotice, Loading } from '../components/Feedback'
import { PageHeading } from '../components/PageHeading'
import { RecordArt } from '../components/RecordArt'
import { artistName } from '../format'
import { PlayerControls } from '../player/PlayerControls'
import { useAudioControls } from '../player/PlayerProvider'
import { usePlayer } from '../player/store'

export function TrackPage() {
  const { trackId = '' } = useParams()
  const location = useLocation()
  const navigate = useNavigate()
  const autoPlay = location.state?.autoPlay === true
  const { play } = useAudioControls()
  const query = useQuery(trackOptions(trackId))
  const select = usePlayer((state) => state.select)
  const selectedId = usePlayer((state) => state.track?.id)
  useEffect(() => { if (query.data) select(query.data) }, [query.data, select])
  useEffect(() => {
    if (!autoPlay || !query.data || selectedId !== query.data.id) return
    play()
    void navigate(location.pathname + location.search + location.hash, { replace: true, state: { ...location.state, autoPlay: false } })
  }, [autoPlay, query.data, selectedId, play, navigate, location.pathname, location.search, location.hash])
  if (query.isPending) return <Loading />
  if (query.isError) return <ErrorNotice error={query.error} retry={() => void query.refetch()} />
  const track = query.data
  const audio = track.originalAudioFile
  return (
    <Stack width="full" maxW="lg" mx="auto" gap="6">
      <Box width={{ base: '48', md: '56' }} alignSelf="center"><RecordArt /></Box>
      <PageHeading title={track.title}><Text color="fg.muted" textStyle="lg">{artistName(track.artistCredits)}</Text></PageHeading>
      {selectedId === track.id && <PlayerControls />}
      <HStack wrap="wrap" justify="center" color="fg.muted" gap="3">
        <AudioFormatBadge mediaType={audio.mediaType} /><Text textStyle="xs">{audio.sampleRate / 1000} kHz{audio.bitDepth !== null && ` · ${audio.bitDepth} bit`} · {audio.channels} ch</Text>
      </HStack>
    </Stack>
  )
}

import { Box, HStack, Stack, Text } from '@chakra-ui/react'
import { useQuery } from '@tanstack/react-query'
import { useEffect } from 'react'
import { useParams } from 'react-router-dom'
import { trackOptions } from '../api/queries'
import { AudioFormatBadge } from '../components/AudioFormatBadge'
import { ErrorNotice, Loading } from '../components/Feedback'
import { PageHeading } from '../components/PageHeading'
import { RecordArt } from '../components/RecordArt'
import { artistName } from '../format'
import { PlayerControls } from '../player/PlayerControls'
import { usePlayer } from '../player/store'

export function TrackPage() {
  const { trackId = '' } = useParams()
  const query = useQuery(trackOptions(trackId))
  const select = usePlayer((state) => state.select)
  const selectedId = usePlayer((state) => state.track?.id)
  useEffect(() => { if (query.data) select(query.data) }, [query.data, select])
  if (query.isPending) return <Loading />
  if (query.isError) return <ErrorNotice error={query.error} retry={() => void query.refetch()} />
  const track = query.data
  const audio = track.originalAudioFile
  return (
    <Stack width="full" maxW="lg" mx="auto" gap="6">
      <Box width={{ base: '48', md: '56' }} alignSelf="center"><RecordArt /></Box>
      <PageHeading title={track.title}>
        <HStack gap="3" align="start" justify="space-between" wrap="wrap">
          <Text color="fg.muted" textStyle="lg" flex="1" minW="0" overflowWrap="anywhere">{artistName(track.artistCredits)}</Text>
          <HStack wrap="wrap" justify="flex-end" color="fg.muted" gap="2" ml="auto" maxW="full">
            <AudioFormatBadge mediaType={audio.mediaType} /><Text textStyle="xs" textAlign="right">{audio.sampleRate / 1000} kHz{audio.bitDepth !== null && ` · ${audio.bitDepth} bit`} · {audio.channels} ch</Text>
          </HStack>
        </HStack>
      </PageHeading>
      {selectedId === track.id && <PlayerControls />}
    </Stack>
  )
}

import { Badge, Box, HStack, Heading, IconButton, Link, Stack, Text } from '@chakra-ui/react'
import { useQuery } from '@tanstack/react-query'
import { ArrowLeft, Disc3, Play } from 'lucide-react'
import { Link as RouterLink, useParams } from 'react-router-dom'
import { releaseOptions, trackOptions } from '../api/queries'
import { AudioFormatBadge } from '../components/AudioFormatBadge'
import { ErrorNotice, Loading } from '../components/Feedback'
import { PageHeading } from '../components/PageHeading'
import { RecordArt } from '../components/RecordArt'
import { useAudioControls } from '../player/PlayerProvider'
import { artistName, formatTime } from '../format'

function TrackFormatBadge({ trackId }: { trackId: string }) {
  const { data } = useQuery(trackOptions(trackId))
  return data ? <AudioFormatBadge mediaType={data.originalAudioFile.mediaType} /> : null
}

function TrackPlayButton({ trackId, title }: { trackId: string; title: string }) {
  const { data, isPending } = useQuery(trackOptions(trackId))
  const { playTrack } = useAudioControls()
  return (
    <IconButton aria-label={`Play ${title}`} variant="ghost" rounded="full" disabled={!data} loading={isPending} onClick={() => { if (data) playTrack(data) }}>
      <Play size={18} />
    </IconButton>
  )
}

export function ReleasePage() {
  const { releaseId = '' } = useParams()
  const query = useQuery(releaseOptions(releaseId))
  if (query.isPending) return <Loading />
  if (query.isError) return <ErrorNotice error={query.error} retry={() => void query.refetch()} />
  const release = query.data
  const totalDurationMs = release.tracks.every((track) => track.durationMs !== null)
    ? release.tracks.reduce((total, track) => total + (track.durationMs ?? 0), 0)
    : null
  const discs = [...new Set(release.tracks.map((track) => track.discNumber))]
  return (
    <Stack gap="10">
      <Link asChild color="fg.muted"><RouterLink to="/" aria-label="Releases" title="リリース一覧に戻る"><ArrowLeft size={16} /></RouterLink></Link>
      <Stack direction={{ base: 'column', md: 'row' }} gap={{ base: '6', md: '10' }} align={{ base: 'stretch', md: 'center' }}>
        <Box width={{ base: '40', md: '64' }} flexShrink="0"><RecordArt /></Box>
        <PageHeading eyebrow="RELEASE" title={release.title}>
          <Text textStyle="lg" color="fg.muted">{artistName(release.artistCredits)}</Text>
          <HStack color="fg.muted" gap="3"><Badge>{release.releaseDate}</Badge><Text textStyle="sm">{release.tracks.length} {release.tracks.length === 1 ? 'track' : 'tracks'}{totalDurationMs !== null && ` · ${formatTime(totalDurationMs / 1000)}`}</Text></HStack>
        </PageHeading>
      </Stack>
      <Stack gap="6">
        <Heading as="h2" size="xl">Tracks</Heading>
        {discs.map((disc) => <Stack as="section" aria-label={`Disc ${disc}`} key={disc} gap="0">
          <HStack py="3" color="fg.muted"><Disc3 size={18} aria-hidden="true" /><Heading as="h3" fontFamily="body" size="sm">Disc {disc}</Heading></HStack>
          {release.tracks.filter((track) => track.discNumber === disc).map((track) => (
            <HStack key={track.id} gap={{ base: '3', md: '5' }} px="3" py="5" borderBottomWidth="1">
              <Text color="fg.muted" width="8" flexShrink="0" textAlign="center">{track.trackNumber}</Text>
              <Stack flex="1" minW="0" gap="1"><Text fontWeight="medium" overflowWrap="anywhere">{track.title}</Text><Text color="fg.muted" textStyle="sm" overflowWrap="anywhere">{artistName(track.artistCredits)}</Text></Stack>
              <HStack gap={{ base: '2', md: '5' }} flexShrink="0">
                <TrackFormatBadge trackId={track.id} />
                <Text textStyle="sm" color="fg.muted" fontVariantNumeric="tabular-nums">{track.durationMs === null ? '—' : formatTime(track.durationMs / 1000)}</Text>
              </HStack>
              <TrackPlayButton trackId={track.id} title={track.title} />
            </HStack>
          ))}
        </Stack>)}
      </Stack>
    </Stack>
  )
}

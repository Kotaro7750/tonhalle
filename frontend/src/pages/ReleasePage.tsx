import { Badge, Box, Grid, HStack, Heading, IconButton, Link, Stack, Text } from '@chakra-ui/react'
import { useQuery } from '@tanstack/react-query'
import { ArrowLeft, Disc3, Play } from '../components/Icons'
import { Link as RouterLink, useParams } from 'react-router-dom'
import { releaseOptions, trackOptions } from '../api/queries'
import { AudioFormatBadge } from '../components/AudioFormatBadge'
import { ErrorNotice, Loading } from '../components/Feedback'
import { PageHeading } from '../components/PageHeading'
import { CollectionActions } from '../player/CollectionActions'
import { QueueAddButton } from '../player/QueueAddButton'
import { RecordArt } from '../components/RecordArt'
import { useAudioControls } from '../player/PlayerProvider'
import { artistName, formatTime } from '../format'

function TrackFormatBadge({ trackId }: { trackId: string }) {
  const { data } = useQuery(trackOptions(trackId))
  return data ? <AudioFormatBadge mediaType={data.originalAudioFile.mediaType} /> : null
}

function TrackActions({ trackId, title }: { trackId: string; title: string }) {
  const { data, isPending } = useQuery(trackOptions(trackId))
  const { playTrack } = useAudioControls()
  return (
    <HStack gap="1">
      <QueueAddButton track={data} title={title} />
      <IconButton aria-label={`Play ${title}`} variant="ghost" rounded="full" disabled={!data} loading={isPending} onClick={() => { if (data) playTrack(data) }}>
        <Play size={18} />
      </IconButton>
    </HStack>
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
      <Grid templateColumns="auto minmax(0, 1fr)" templateAreas={{ base: '"title title" "art metadata"', md: '"art title" "art metadata"' }} columnGap={{ base: '5', md: '10' }} rowGap={{ base: '6', md: '3' }} alignItems="center">
        <Box gridArea="title" alignSelf={{ md: 'end' }} minW="0">
          <PageHeading title={release.title}>
            <Text textStyle="lg" color="fg.muted">{artistName(release.artistCredits)}</Text>
          </PageHeading>
        </Box>
        <Box gridArea="art" width={{ base: '32', sm: '40', md: '64' }}><RecordArt /></Box>
        <Stack gridArea="metadata" gap="3" minW="0" alignSelf={{ md: 'start' }}>
          <Stack direction={{ base: 'column', md: 'row' }} align="start" color="fg.muted" gap={{ base: '2', md: '3' }}><Badge>{release.releaseDate}</Badge><Text textStyle="sm">{release.tracks.length} {release.tracks.length === 1 ? 'track' : 'tracks'}{totalDurationMs !== null && ` · ${formatTime(totalDurationMs / 1000)}`}</Text></Stack>
          <CollectionActions releaseId={release.id} title={release.title} />
        </Stack>
      </Grid>
      <Stack gap="6">
        <Heading as="h2" size="xl">Tracks</Heading>
        {discs.map((disc) => <Stack as="section" aria-label={`Disc ${disc}`} key={disc} gap="0">
          <HStack py="3" color="fg.muted"><Disc3 size={18} aria-hidden="true" /><Heading as="h3" fontFamily="body" size="sm">Disc {disc}</Heading><CollectionActions releaseId={release.id} title={release.title} discNumber={disc} /></HStack>
          {release.tracks.filter((track) => track.discNumber === disc).map((track) => (
            <Grid key={track.id} templateColumns={{ base: 'auto minmax(0, 1fr) auto', md: 'auto minmax(0, 1fr) auto auto' }} templateAreas={{ base: '"number title actions" "number metadata actions"', md: '"number title metadata actions"' }} columnGap={{ base: '3', md: '5' }} rowGap="3" px={{ base: '0', md: '3' }} py="5" borderBottomWidth="1" alignItems="center">
              <Text gridArea="number" color="fg.muted" width={{ base: '5', md: '8' }} textAlign="center" alignSelf={{ base: 'start', md: 'center' }}>{track.trackNumber}</Text>
              <Stack gridArea="title" minW="0" gap="1"><Text fontWeight="medium" overflowWrap="anywhere">{track.title}</Text><Text color="fg.muted" textStyle="sm" overflowWrap="anywhere">{artistName(track.artistCredits)}</Text></Stack>
              <HStack gridArea="metadata" gap={{ base: '3', md: '5' }}>
                <TrackFormatBadge trackId={track.id} />
                <Text textStyle="sm" color="fg.muted" fontVariantNumeric="tabular-nums">{track.durationMs === null ? '—' : formatTime(track.durationMs / 1000)}</Text>
              </HStack>
              <Box gridArea="actions"><TrackActions trackId={track.id} title={track.title} /></Box>
            </Grid>
          ))}
        </Stack>)}
      </Stack>
    </Stack>
  )
}

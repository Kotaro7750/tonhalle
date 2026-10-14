import { Badge, Box, HStack, Heading, IconButton, Link, Stack, Text } from '@chakra-ui/react'
import { useQuery } from '@tanstack/react-query'
import { ArrowLeft, Disc3, Play } from 'lucide-react'
import { Link as RouterLink, useLocation, useNavigate, useParams } from 'react-router-dom'
import { releaseOptions } from '../api/queries'
import { ErrorNotice, Loading } from '../components/Feedback'
import { PageHeading } from '../components/PageHeading'
import { RecordArt } from '../components/RecordArt'
import { artistName, formatTime } from '../format'

export function ReleasePage() {
  const { releaseId = '' } = useParams()
  const navigate = useNavigate()
  const location = useLocation()
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
      <Link asChild color="fg.muted"><RouterLink to="/"><ArrowLeft size={16} />Releases</RouterLink></Link>
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
              <Text textStyle="sm" color="fg.muted" fontVariantNumeric="tabular-nums">{track.durationMs === null ? '—' : formatTime(track.durationMs / 1000)}</Text>
              <IconButton aria-label={`Play ${track.title}`} variant="ghost" rounded="full" onClick={() => navigate(`/tracks/${track.id}`, { state: { autoPlay: true, backgroundLocation: location } })}>
                <Play size={18} />
              </IconButton>
            </HStack>
          ))}
        </Stack>)}
      </Stack>
    </Stack>
  )
}

import { Heading, Link, Stack, Text } from '@chakra-ui/react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { Link as RouterLink, useParams } from 'react-router-dom'
import { artistCollectionOptions, artistOptions } from '../api/queries'
import { ArrowLeft } from '../components/Icons'
import { ErrorNotice, Loading } from '../components/Feedback'
import { PageHeading } from '../components/PageHeading'
import { ReleasesPage } from './ReleasesPage'
import { DetailHeader } from '../components/DetailHeader'
import { CollectionActions } from '../player/CollectionActions'
import { formatTime } from '../format'

function ArtistMetadata({ artistId, title }: { artistId: string; title: string }) {
  const client = useQueryClient()
  const query = useQuery(artistCollectionOptions(artistId, client))
  const collection = query.data
  const duration = collection?.tracks.every((track) => track.durationMs !== null)
    ? collection.tracks.reduce((total, track) => total + (track.durationMs ?? 0), 0)
    : null
  return <>
    {query.isPending && <Text textStyle="sm" color="fg.muted" role="status">Loading collection details…</Text>}
    {query.isError && !collection && <ErrorNotice error={query.error} retry={() => void query.refetch()} />}
    {collection && <Text textStyle="sm" color="fg.muted">{collection.releaseCount} {collection.releaseCount === 1 ? 'release' : 'releases'} · {collection.tracks.length} {collection.tracks.length === 1 ? 'track' : 'tracks'}{duration !== null && ` · ${formatTime(duration / 1000)}`}</Text>}
    <CollectionActions artistId={artistId} title={title} />
  </>
}

export function ArtistPage() {
  const { artistId = '' } = useParams()
  const query = useQuery(artistOptions(artistId))
  if (query.isPending) return <Loading />
  if (query.isError) return <ErrorNotice error={query.error} retry={() => void query.refetch()} />
  return <Stack gap="10">
    <Link asChild color="fg.muted"><RouterLink to="/?view=artists" aria-label="Artists" title="アーティスト一覧に戻る"><ArrowLeft size={16} /></RouterLink></Link>
    <DetailHeader metadata={<ArtistMetadata key={artistId} artistId={artistId} title={query.data.name} />}><PageHeading title={query.data.name} /></DetailHeader>
    <Stack gap="6">
      <Heading as="h2" size="xl">Releases</Heading>
      <ReleasesPage key={artistId} artistId={artistId} />
    </Stack>
  </Stack>
}

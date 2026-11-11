import { Link, Stack } from '@chakra-ui/react'
import { useQuery } from '@tanstack/react-query'
import { Link as RouterLink, useParams } from 'react-router-dom'
import { artistOptions } from '../api/queries'
import { ArrowLeft } from '../components/Icons'
import { ErrorNotice, Loading } from '../components/Feedback'
import { PageHeading } from '../components/PageHeading'
import { ReleasesPage } from './ReleasesPage'

export function ArtistPage() {
  const { artistId = '' } = useParams()
  const query = useQuery(artistOptions(artistId))
  if (query.isPending) return <Loading />
  if (query.isError) return <ErrorNotice error={query.error} retry={() => void query.refetch()} />
  return <Stack gap="10">
    <Link asChild color="fg.muted"><RouterLink to="/?view=artists" aria-label="Artists" title="アーティスト一覧に戻る"><ArrowLeft size={16} /></RouterLink></Link>
    <PageHeading title={query.data.name} eyebrow="RELEASES" />
    <ReleasesPage key={artistId} artistId={artistId} />
  </Stack>
}

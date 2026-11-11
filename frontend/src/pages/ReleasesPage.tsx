import { Stack } from '@chakra-ui/react'
import { useInfiniteQuery } from '@tanstack/react-query'
import { releasesByArtistOptions } from '../api/queries'
import { PagedCollection } from '../components/PagedCollection'
import { PageHeading } from '../components/PageHeading'
import { ReleaseGrid } from '../components/ReleaseGrid'

export function ReleasesPage({ artistId }: { artistId?: string }) {
  const query = useInfiniteQuery(releasesByArtistOptions(artistId))
  return (
    <Stack gap="10">
      {!artistId && <PageHeading title="Releases" />}
      <PagedCollection query={query} label="releases">
        {(items) => <ReleaseGrid releases={items} />}
      </PagedCollection>
    </Stack>
  )
}

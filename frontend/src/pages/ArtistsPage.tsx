import { Box, Link, Stack, Text } from '@chakra-ui/react'
import { useInfiniteQuery } from '@tanstack/react-query'
import { Link as RouterLink } from 'react-router-dom'
import { artistListOptions } from '../api/queries'
import { PagedCollection } from '../components/PagedCollection'
import { PageHeading } from '../components/PageHeading'
import { RecordArt } from '../components/RecordArt'

export function ArtistsPage() {
  const query = useInfiniteQuery(artistListOptions)
  return <Stack gap="10">
    <PageHeading title="Artists" />
    <PagedCollection query={query} label="artists">{(artists) => artists.length ? (
      <Stack as="ul" aria-label="Release artists" gap="2" listStyleType="none" m="0" p="0">
        {artists.map((artist) => <Box as="li" key={artist.id}>
          <Link asChild display="flex" alignItems="center" gap={{ base: '4', md: '5' }} p="3" rounded="xl" color="fg" textDecoration="none" _hover={{ bg: 'bg.subtle' }}>
            <RouterLink to={`/artists/${artist.id}`}>
              <Box width={{ base: '20', md: '24' }} flexShrink="0"><RecordArt /></Box>
              <Text fontWeight="medium" minW="0" overflowWrap="anywhere">{artist.name}</Text>
            </RouterLink>
          </Link>
        </Box>)}
      </Stack>
    ) : <Text color="fg.muted">No release artists yet.</Text>}</PagedCollection>
  </Stack>
}

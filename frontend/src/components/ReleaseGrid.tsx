import { Card, Heading, Link, SimpleGrid, Stack, Text } from '@chakra-ui/react'
import { Link as RouterLink } from 'react-router-dom'
import type { ReleaseSummary } from '../api/generated'
import { EmptyLibrary } from './Feedback'
import { RecordArt } from './RecordArt'
import { CollectionActions } from '../player/CollectionActions'
import { artistName } from '../format'

export function ReleaseGrid({ releases }: { releases: ReleaseSummary[] }) {
  return (
    !releases.length ? <EmptyLibrary /> : <SimpleGrid columns={{ base: 2, md: 3, lg: 4 }} gap={{ base: '5', md: '8' }}>
      {releases.map((release) => <Card.Root key={release.id} variant="outline" bg="bg.panel" overflow="hidden">
        <Link asChild display="block" flex="1" color="fg" textDecoration="none" _hover={{ bg: 'bg.subtle' }}>
          <RouterLink to={`/releases/${release.id}`}>
            <Card.Body gap="4" p={{ base: '3', md: '5' }}>
              <RecordArt />
              <Stack gap="1">
                <Heading as="h2" size="md" fontFamily="body" overflowWrap="anywhere">{release.title}</Heading>
                <Text textStyle="sm" color="fg.muted" overflowWrap="anywhere">{artistName(release.artistCredits)}</Text>
                {release.releaseDate !== null && <Text textStyle="xs" color="fg.muted">{release.releaseDate}</Text>}
              </Stack>
            </Card.Body>
          </RouterLink>
        </Link>
        <Card.Footer justifyContent="flex-end" px={{ base: '3', md: '5' }} py="2"><CollectionActions releaseId={release.id} title={release.title} /></Card.Footer>
      </Card.Root>)}
    </SimpleGrid>
  )
}

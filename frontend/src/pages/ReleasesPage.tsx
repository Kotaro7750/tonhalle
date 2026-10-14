import { Box, Button, Card, Heading, Link, SimpleGrid, Stack, Text } from '@chakra-ui/react'
import { useInfiniteQuery } from '@tanstack/react-query'
import { ArrowDown } from 'lucide-react'
import { useEffect, useRef } from 'react'
import { Link as RouterLink } from 'react-router-dom'
import { releaseListOptions } from '../api/queries'
import { EmptyLibrary, ErrorNotice, Loading } from '../components/Feedback'
import { PageHeading } from '../components/PageHeading'
import { RecordArt } from '../components/RecordArt'
import { artistName } from '../format'

export function ReleasesPage() {
  const query = useInfiniteQuery(releaseListOptions)
  const sentinel = useRef<HTMLDivElement>(null)
  const { hasNextPage, isFetchingNextPage, isFetchNextPageError, fetchNextPage } = query

  useEffect(() => {
    if (!sentinel.current || !hasNextPage || isFetchingNextPage || isFetchNextPageError) return
    const observer = new IntersectionObserver(([entry]) => {
      if (entry.isIntersecting) void fetchNextPage()
    }, { rootMargin: '240px' })
    observer.observe(sentinel.current)
    return () => observer.disconnect()
  }, [hasNextPage, isFetchingNextPage, isFetchNextPageError, fetchNextPage])

  const items = query.data?.pages.flatMap((page) => page.items) ?? []
  const uniqueItems = [...new Map(items.map((item) => [item.id, item])).values()]
  return (
    <Stack gap="10">
      <PageHeading title="Releases" />
      {query.isPending ? <Loading /> : query.isError && !query.data ? <ErrorNotice error={query.error} retry={() => void query.refetch()} /> : (
        <>
          {!uniqueItems.length ? <EmptyLibrary /> : <SimpleGrid columns={{ base: 2, md: 3, lg: 4 }} gap={{ base: '5', md: '8' }}>
            {uniqueItems.map((release) => <Card.Root key={release.id} variant="outline" bg="bg.panel" overflow="hidden">
              <Link asChild display="block" height="full" color="fg" textDecoration="none" _hover={{ bg: 'bg.subtle' }}>
                <RouterLink to={`/releases/${release.id}`}>
                  <Card.Body gap="4" p={{ base: '3', md: '5' }}>
                    <RecordArt />
                    <Stack gap="1">
                      <Heading as="h2" size="md" fontFamily="body" overflowWrap="anywhere">{release.title}</Heading>
                      <Text textStyle="sm" color="fg.muted" overflowWrap="anywhere">{artistName(release.artistCredits)}</Text>
                      <Text textStyle="xs" color="fg.muted">{release.releaseDate}</Text>
                    </Stack>
                  </Card.Body>
                </RouterLink>
              </Link>
            </Card.Root>)}
          </SimpleGrid>}
          <Box ref={sentinel}>
            {isFetchNextPageError && <ErrorNotice error={query.error!} retry={() => void fetchNextPage()} />}
            {hasNextPage && !isFetchNextPageError && <Button width="full" variant="ghost" loading={isFetchingNextPage} loadingText="読み込み中…" onClick={() => void fetchNextPage()}><ArrowDown size={16} />さらに読み込む</Button>}
            {!hasNextPage && !!uniqueItems.length && <Text textAlign="center" textStyle="sm" color="fg.muted">All releases loaded</Text>}
          </Box>
        </>
      )}
    </Stack>
  )
}

import { Box, IconButton, Text } from '@chakra-ui/react'
import type { InfiniteData, UseInfiniteQueryResult } from '@tanstack/react-query'
import { useEffect, useRef, type ReactNode } from 'react'
import { ArrowDown } from './Icons'
import { ErrorNotice, Loading } from './Feedback'

type Page<T> = { items: T[]; nextCursor?: string }

export function PagedCollection<T extends { id: string }>({ query, label, children }: {
  query: UseInfiniteQueryResult<InfiniteData<Page<T>>, Error>
  label: string
  children: (items: T[]) => ReactNode
}) {
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

  if (query.isPending) return <Loading />
  if (query.isError && !query.data) return <ErrorNotice error={query.error} retry={() => void query.refetch()} />
  const items = query.data?.pages.flatMap((page) => page.items) ?? []
  const uniqueItems = [...new Map(items.map((item) => [item.id, item])).values()]
  return <>
    {children(uniqueItems)}
    <Box ref={sentinel}>
      {isFetchNextPageError && <ErrorNotice error={query.error!} retry={() => void fetchNextPage()} />}
      {hasNextPage && !isFetchNextPageError && <IconButton aria-label="Load more" title="さらに読み込む" width="full" variant="ghost" loading={isFetchingNextPage} onClick={() => void fetchNextPage()}><ArrowDown size={16} /></IconButton>}
      {!hasNextPage && !!uniqueItems.length && <Text textAlign="center" textStyle="sm" color="fg.muted">All {label} loaded</Text>}
    </Box>
  </>
}

import { Tabs } from '@chakra-ui/react'
import { useEffect } from 'react'
import { Link as RouterLink, useSearchParams } from 'react-router-dom'
import { ArtistsPage } from './ArtistsPage'
import { ReleasesPage } from './ReleasesPage'
import { useLibrary } from '../library/store'

export function LibraryPage() {
  const [params, setParams] = useSearchParams()
  const savedView = useLibrary((state) => state.view)
  const setView = useLibrary((state) => state.setView)
  const requestedView = params.get('view')
  const view = requestedView === 'artists' || requestedView === 'releases' ? requestedView : savedView
  useEffect(() => { setView(view) }, [view, setView])
  return <Tabs.Root value={view} onValueChange={({ value }) => {
    const nextView = value === 'artists' ? 'artists' : 'releases'
    setView(nextView)
    setParams({ view: nextView })
  }} variant="line" lazyMount unmountOnExit>
    <Tabs.List aria-label="Library view" width="fit-content" gap="6">
      <Tabs.Trigger asChild value="releases" px="0" pb="3" color="fg.muted" _selected={{ color: 'fg', borderColor: 'colorPalette.fg' }}>
        <RouterLink to="/?view=releases">Releases</RouterLink>
      </Tabs.Trigger>
      <Tabs.Trigger asChild value="artists" px="0" pb="3" color="fg.muted" _selected={{ color: 'fg', borderColor: 'colorPalette.fg' }}>
        <RouterLink to="/?view=artists">Artists</RouterLink>
      </Tabs.Trigger>
    </Tabs.List>
    <Tabs.Content value="releases" pt="8"><ReleasesPage /></Tabs.Content>
    <Tabs.Content value="artists" pt="8"><ArtistsPage /></Tabs.Content>
  </Tabs.Root>
}

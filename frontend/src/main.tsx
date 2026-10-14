import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { HashRouter } from 'react-router-dom'
import { App } from './App'
import { retryRequest } from './api/queries'
import { Provider } from './components/ui/provider'
import { PlayerProvider } from './player/PlayerProvider'

const queryClient = new QueryClient({
  defaultOptions: { queries: { staleTime: 60_000, retry: retryRequest } },
})

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <Provider>
      <QueryClientProvider client={queryClient}>
        <HashRouter>
          <PlayerProvider><App /></PlayerProvider>
        </HashRouter>
      </QueryClientProvider>
    </Provider>
  </StrictMode>,
)

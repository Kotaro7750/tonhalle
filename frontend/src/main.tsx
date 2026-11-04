import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { HashRouter } from 'react-router-dom'
import { AuthBoundary } from './auth/AuthBoundary'
import { ProtectedRoute } from './auth/ProtectedRoute'
import { ContentAccessBoundary } from './content-access/ContentAccessBoundary'
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
        <AuthBoundary>
          <HashRouter>
            <ProtectedRoute>
              <ContentAccessBoundary>
                <PlayerProvider><App /></PlayerProvider>
              </ContentAccessBoundary>
            </ProtectedRoute>
          </HashRouter>
        </AuthBoundary>
      </QueryClientProvider>
    </Provider>
  </StrictMode>,
)

import { Box, ChakraProvider } from '@chakra-ui/react'
import { ThemeProvider } from 'next-themes'
import type { ReactNode } from 'react'
import { system } from '../../theme'

export function Provider({ children }: { children: ReactNode }) {
  return (
    <ChakraProvider value={system}>
      {/* The SSR bootstrap script stays inert in this client-rendered SPA. */}
      <ThemeProvider
        attribute="class"
        defaultTheme="system"
        storageKey="tonhalle-theme"
        disableTransitionOnChange
        scriptProps={{ type: 'text/plain' }}
      >
        <Box colorPalette="teal" minH="100dvh">{children}</Box>
      </ThemeProvider>
    </ChakraProvider>
  )
}

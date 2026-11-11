import { Box, Grid, Stack } from '@chakra-ui/react'
import type { ReactNode } from 'react'
import { RecordArt } from './RecordArt'

export function DetailHeader({ children, metadata }: { children: ReactNode; metadata?: ReactNode }) {
  return (
    <Grid templateColumns="auto minmax(0, 1fr)" templateAreas={metadata ? { base: '"title title" "art metadata"', md: '"art title" "art metadata"' } : { base: '"title title" "art art"', md: '"art title"' }} columnGap={{ base: '5', md: '10' }} rowGap={{ base: '6', md: '3' }} alignItems="center">
      <Box gridArea="title" alignSelf={metadata ? { md: 'end' } : undefined} minW="0">{children}</Box>
      <Box gridArea="art" width={{ base: '32', sm: '40', md: '64' }}><RecordArt /></Box>
      {metadata && <Stack gridArea="metadata" gap="3" minW="0" alignSelf={{ md: 'start' }}>{metadata}</Stack>}
    </Grid>
  )
}

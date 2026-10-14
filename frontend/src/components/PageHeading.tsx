import { Heading, Stack, Text } from '@chakra-ui/react'
import type { ReactNode } from 'react'

export function PageHeading({ eyebrow, title, children }: { eyebrow?: string; title: string; children?: ReactNode }) {
  return (
    <Stack gap="3">
      {eyebrow && <Text textStyle="xs" fontWeight="semibold" letterSpacing="widest" color="fg.muted">{eyebrow}</Text>}
      <Heading as="h1" size={{ base: '3xl', md: '4xl' }} overflowWrap="anywhere">{title}</Heading>
      {children}
    </Stack>
  )
}

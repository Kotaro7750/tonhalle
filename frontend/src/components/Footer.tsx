import { Box, Container, HStack, Link, Text } from '@chakra-ui/react'
import { Github } from './Icons'
import { version } from '../../package.json'

export function Footer() {
  return (
    <Box as="footer" bg="colorPalette.subtle" borderTopWidth="1" borderColor="colorPalette.muted">
      <Container maxW="6xl" px={{ base: '5', md: '8' }} py="2">
        <HStack justify="flex-end" gap="4" color="colorPalette.fg">
          <Text textStyle="sm">v{version}</Text>
          <Link href="https://github.com/Kotaro7750/tonhalle" target="_blank" rel="noopener noreferrer" aria-label="GitHub repository" title="GitHubリポジトリを開く" color="colorPalette.fg" p="2" rounded="md">
            <Github size={20} aria-hidden="true" />
          </Link>
        </HStack>
      </Container>
    </Box>
  )
}

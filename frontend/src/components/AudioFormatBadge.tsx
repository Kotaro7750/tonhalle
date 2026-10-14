import { Badge } from '@chakra-ui/react'
import type { AudioFile } from '../api/generated'

export function AudioFormatBadge({ mediaType }: { mediaType: AudioFile['mediaType'] }) {
  const label = { 'audio/wav': 'WAV', 'audio/flac': 'FLAC', 'audio/aac': 'AAC', 'audio/mp4': 'M4A', 'audio/ogg': 'OGG', 'audio/mpeg': 'MP3' }[mediaType]
  return <Badge variant="outline" flexShrink="0">{label}</Badge>
}

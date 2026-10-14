import { AspectRatio, Center } from '@chakra-ui/react'
import { Disc3 } from 'lucide-react'

export function RecordArt() {
  return (
    <AspectRatio ratio={1} width="full" aria-hidden="true">
      <Center bg="bg.muted" rounded="xl" color="fg.muted">
        <Disc3 width="45%" height="45%" strokeWidth={0.8} />
      </Center>
    </AspectRatio>
  )
}

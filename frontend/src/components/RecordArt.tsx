import { AspectRatio, Center } from '@chakra-ui/react'
import { Disc3 } from './Icons'

export function RecordArt() {
  return (
    <AspectRatio ratio={1} width="full" aria-hidden="true">
      <Center bg="bg.muted" rounded="xl" color="fg.muted">
        <Disc3 size="45%" />
      </Center>
    </AspectRatio>
  )
}

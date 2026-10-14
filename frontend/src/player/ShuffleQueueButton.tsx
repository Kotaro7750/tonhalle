import { IconButton } from '@chakra-ui/react'
import { Shuffle } from '../components/Icons'
import { useAudioControls } from './PlayerProvider'
import { usePlayer } from './store'

export function ShuffleQueueButton() {
  const empty = usePlayer((state) => !state.track && !state.history.length && !state.queue.length)
  const { shuffleQueue } = useAudioControls()
  return <IconButton aria-label="Shuffle and play queue" title="キューをシャッフルして再生" variant="ghost" rounded="full" disabled={empty} onClick={shuffleQueue}><Shuffle size={18} /></IconButton>
}

import { IconButton } from '@chakra-ui/react'
import { ListCheck, ListPlus } from '../components/Icons'
import type { Track } from '../api/generated'
import { usePlayer } from './store'

export function QueueAddButton({ track, title }: { track?: Track; title: string }) {
  const queued = usePlayer((state) => !!track && (state.track?.id === track.id || state.history.some((item) => item.id === track.id) || state.queue.some((item) => item.id === track.id)))
  const enqueue = usePlayer((state) => state.enqueue)
  return <IconButton aria-label={`${queued ? 'Queued' : 'Add to queue'}: ${title}`} title={queued ? '再生中またはキューに追加済み' : 'キューに追加'} variant="ghost" rounded="full" disabled={!track || queued} onClick={() => { if (track) enqueue(track) }}>{queued ? <ListCheck size={18} /> : <ListPlus size={18} />}</IconButton>
}

import { HStack, IconButton, Stack, Text } from '@chakra-ui/react'
import { useSortable } from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'
import { GripVertical, Play, Trash2 } from '../components/Icons'
import type { ReactNode } from 'react'
import type { Track } from '../api/generated'
import { artistName, formatTime } from '../format'

export function QueueTrackContent({ track, status = 'upcoming', handle, onPlay, onRemove }: {
  track: Track; status?: 'played' | 'current' | 'upcoming'; handle?: ReactNode; onPlay?: () => void; onRemove?: () => void
}) {
  return <HStack gap={{ base: '3', md: '5' }} py="5" px="2" borderBottomWidth="1" bg={status === 'current' ? 'colorPalette.subtle' : status === 'played' ? 'bg.muted' : 'bg.panel'} width="full">
    {handle ?? <IconButton variant="ghost" size="sm" aria-label="Reorder track" disabled><GripVertical size={16} /></IconButton>}
    <Stack flex="1" minW="0" gap="1"><Text fontWeight={status === 'current' ? 'semibold' : 'medium'} overflowWrap="anywhere">{track.title}</Text><Text color="fg.muted" textStyle="sm" overflowWrap="anywhere">{artistName(track.artistCredits)}</Text></Stack>
    <Text color="fg.muted" textStyle="sm" fontVariantNumeric="tabular-nums">{track.durationMs === null ? '—' : formatTime(track.durationMs / 1000)}</Text>
    <HStack gap="1" flexShrink="0">
      <IconButton aria-label={`Play ${track.title}`} title="この曲を再生" variant="ghost" rounded="full" onClick={onPlay} disabled={!onPlay || status === 'current'}><Play size={18} /></IconButton>
      <IconButton aria-label={`Remove from queue: ${track.title}`} title={status === 'current' ? '再生を停止してキューから削除' : 'キューから削除'} variant="ghost" rounded="full" onClick={onRemove} disabled={!onRemove}><Trash2 size={18} /></IconButton>
    </HStack>
  </HStack>
}

export function QueueTrackRow({ track, status = 'upcoming', onPlay, onRemove }: {
  track: Track; status?: 'played' | 'current' | 'upcoming'; onPlay: () => void; onRemove: () => void
}) {
  const { attributes, listeners, setNodeRef, setActivatorNodeRef, transform, transition, isDragging } = useSortable({ id: track.id })
  return <Stack as="li" data-queue-state={status} aria-current={status === 'current' ? 'true' : undefined} aria-label={status === 'current' ? `Current track: ${track.title}` : status === 'played' ? `Previously played: ${track.title}` : undefined} ref={setNodeRef} gap="0" cursor="grab" touchAction="none" _active={{ cursor: 'grabbing' }}
    onPointerDown={(event) => {
      if ((event.target as HTMLElement).closest('button')) return
      listeners?.onPointerDown?.(event)
    }} opacity={isDragging ? 0.3 : 1}
    style={{ transform: CSS.Transform.toString(transform), transition }} _motionReduce={{ transition: 'none' }}>
    <QueueTrackContent track={track} status={status} onPlay={onPlay} onRemove={onRemove} handle={
      <IconButton ref={setActivatorNodeRef} {...attributes} {...listeners} aria-label={`Reorder ${track.title}`} title="ドラッグして並べ替え" variant="ghost" size="sm" cursor="grab" touchAction="none" _active={{ cursor: 'grabbing' }}><GripVertical size={16} /></IconButton>
    } />
  </Stack>
}

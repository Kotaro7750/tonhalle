import { Badge, CloseButton, Drawer, HStack, IconButton, Portal, Stack, Text } from '@chakra-ui/react'
import { ListMusic, Play, Trash2 } from '../components/Icons'
import { useState } from 'react'
import { DndContext, DragOverlay, KeyboardSensor, PointerSensor, closestCenter, useSensor, useSensors } from '@dnd-kit/core'
import { SortableContext, sortableKeyboardCoordinates, verticalListSortingStrategy } from '@dnd-kit/sortable'
import { QueueTrackContent, QueueTrackRow } from './QueueTrackRow'
import { PlayerHandle } from './PlayerHandle'
import { useAudioControls } from './PlayerProvider'
import { usePlayer } from './store'
import { ShuffleQueueButton } from './ShuffleQueueButton'

export function QueueButton() {
  const update = usePlayer((state) => state.update)
  const count = usePlayer((state) => state.history.length + state.queue.length + (state.track ? 1 : 0))
  return <IconButton aria-label="Open queue" title="キューを表示" variant="ghost" rounded="full" data-queue-trigger position="relative" onClick={() => update({ queueOpen: true })}><ListMusic /><Badge aria-label={`${count} ${count === 1 ? 'track' : 'tracks'} in queue`} position="absolute" top="-1" right="-1" size="xs" rounded="full" minW="4" justifyContent="center">{count}</Badge></IconButton>
}

export function QueueOverlay() {
  const { history, queue, queueOpen, track, update, moveQueuedTrack } = usePlayer()
  const { playQueuedTrack, clearQueue, playQueue, removeQueuedTrack } = useAudioControls()
  const [activeId, setActiveId] = useState<string | null>(null)
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 6 } }),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates }),
  )
  const tracks = [...history, ...(track ? [track] : []), ...queue]
  const status = (id: string) => id === track?.id ? 'current' : history.some((item) => item.id === id) ? 'played' : 'upcoming'
  const activeTrack = tracks.find((item) => item.id === activeId)
  const close = () => update({ queueOpen: false })
  return (
    <Drawer.Root open={queueOpen} onOpenChange={({ open }) => update({ queueOpen: open })} placement="bottom" lazyMount unmountOnExit finalFocusEl={() => document.querySelector<HTMLElement>('[data-queue-trigger]')}>
      <Portal>
        <Drawer.Backdrop _motionReduce={{ animation: 'none' }} />
        <Drawer.Positioner>
          <Drawer.Content maxW="6xl" mx="auto" h="92dvh" roundedTop="2xl" overflow="hidden" _motionReduce={{ animation: 'none' }}>
            <PlayerHandle label="Collapse queue" onActivate={close} onUp={() => {}} onDown={close} />
            <Drawer.Header px={{ base: '5', md: '8' }}><HStack justify="space-between" w="full" maxW="3xl" mx="auto" px="2"><Drawer.Title>Queue</Drawer.Title><HStack gap="1"><IconButton aria-label="Play queue from start" title="キューの先頭から再生" variant="ghost" rounded="full" disabled={!track && history.length === 0 && queue.length === 0} onClick={playQueue}><Play size={18} /></IconButton><ShuffleQueueButton /><IconButton aria-label="Clear queue" title="キューをクリアして再生を停止" variant="ghost" rounded="full" disabled={!track && history.length === 0 && queue.length === 0} onClick={clearQueue}><Trash2 size={18} /></IconButton></HStack></HStack></Drawer.Header>
            <Drawer.CloseTrigger asChild><CloseButton aria-label="Close queue" /></Drawer.CloseTrigger>
            <Drawer.Body px={{ base: '5', md: '8' }} pb="8" minH="0">
              <Stack maxW="3xl" mx="auto">
                <DndContext sensors={sensors} collisionDetection={closestCenter}
                  onDragStart={({ active }) => setActiveId(String(active.id))}
                  onDragCancel={() => setActiveId(null)}
                  onDragEnd={({ active, over }) => {
                    if (over) moveQueuedTrack(String(active.id), tracks.findIndex((item) => item.id === over.id))
                    setActiveId(null)
                  }}>
                  {history.length === 0 && !track && queue.length === 0 ? <Text color="fg.muted" py="8">Your queue is empty.</Text> :
                    <SortableContext items={tracks.map((item) => item.id)} strategy={verticalListSortingStrategy}>
                      <Stack as="ol" aria-label="Queued tracks" listStyleType="none" gap="0" m="0" p="0" rounded="lg" overflow="hidden">
                        {tracks.map((item) => <QueueTrackRow key={item.id} track={item} status={status(item.id)} onPlay={() => playQueuedTrack(item)} onRemove={() => removeQueuedTrack(item.id)} />)}
                      </Stack>
                    </SortableContext>}
                  <DragOverlay>
                    {activeTrack && <Stack gap="0" bg="bg.panel" shadow="lg" rounded="lg" overflow="hidden" cursor="grabbing" aria-hidden="true" data-queue-drag-overlay>
                      <QueueTrackContent track={activeTrack} status={status(activeTrack.id)} />
                    </Stack>}
                  </DragOverlay>
                </DndContext>
              </Stack>
            </Drawer.Body>
          </Drawer.Content>
        </Drawer.Positioner>
      </Portal>
    </Drawer.Root>
  )
}

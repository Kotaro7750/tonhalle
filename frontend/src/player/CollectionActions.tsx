import { HStack, IconButton, Stack, Text } from '@chakra-ui/react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { ListCheck, ListPlus, Play } from '../components/Icons'
import { useState } from 'react'
import { releaseOptions, trackOptions } from '../api/queries'
import { useAudioControls } from './PlayerProvider'
import { usePlayer } from './store'

export function CollectionActions({ releaseId, title, discNumber }: { releaseId: string; title: string; discNumber?: number }) {
  const client = useQueryClient()
  const { playTracks } = useAudioControls()
  const queue = usePlayer((state) => state.queue)
  const history = usePlayer((state) => state.history)
  const current = usePlayer((state) => state.track)
  const hasReleaseTracks = current?.releaseId === releaseId || history.some((track) => track.releaseId === releaseId) || queue.some((track) => track.releaseId === releaseId)
  const releaseQuery = useQuery({ ...releaseOptions(releaseId), enabled: hasReleaseTracks })
  const members = releaseQuery.data?.tracks.filter((track) => discNumber === undefined || track.discNumber === discNumber) ?? []
  const queuedIds = new Set([current?.id, ...history.map((track) => track.id), ...queue.map((track) => track.id)])
  const queued = members.length > 0 && members.every((track) => queuedIds.has(track.id))
  const enqueueMany = usePlayer((state) => state.enqueueMany)
  const [pending, setPending] = useState<'add' | 'play' | null>(null)
  const [error, setError] = useState<string | null>(null)
  const label = discNumber === undefined ? `release: ${title}` : `disc ${discNumber}`
  const run = async (operation: 'add' | 'play') => {
    if (pending) return
    setPending(operation)
    setError(null)
    try {
      const release = await client.fetchQuery(releaseOptions(releaseId))
      const summaries = release.tracks.filter((track) => discNumber === undefined || track.discNumber === discNumber)
        .sort((a, b) => a.discNumber - b.discNumber || a.trackNumber - b.trackNumber)
      const tracks = await Promise.all(summaries.map((track) => client.fetchQuery(trackOptions(track.id))))
      if (operation === 'play') playTracks(tracks)
      else enqueueMany(tracks)
    } catch (error) {
      setError(error instanceof Error ? error.message : 'Could not load tracks. Try again.')
    } finally {
      setPending(null)
    }
  }
  return <Stack gap="2">
    <HStack gap="1">
      <IconButton aria-label={queued ? `Queued ${label}` : `Add ${label} to queue`} title={queued ? '全曲キューに追加済み' : discNumber === undefined ? 'リリース全体をキューに追加' : 'ディスク全体をキューに追加'} variant="ghost" rounded="full" loading={pending === 'add'} disabled={pending !== null || queued} onClick={() => void run('add')}>{queued ? <ListCheck size={18} /> : <ListPlus size={18} />}</IconButton>
      <IconButton aria-label={`Play ${label}`} title={discNumber === undefined ? 'リリース全体を先頭から再生' : 'ディスク全体を先頭から再生'} variant="ghost" rounded="full" loading={pending === 'play'} disabled={pending !== null} onClick={() => void run('play')}><Play size={18} /></IconButton>
    </HStack>
    {error && <Text role="alert" color="fg.error" textStyle="sm">{error}</Text>}
  </Stack>
}

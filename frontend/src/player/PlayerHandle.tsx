import { Box, Button } from '@chakra-ui/react'
import { ChevronsUp } from 'lucide-react'
import { useRef, useState } from 'react'

export function PlayerHandle({ label, minimized = false, onActivate, onUp, onDown }: {
  label: string
  minimized?: boolean
  onActivate: () => void
  onUp: () => void
  onDown: () => void
}) {
  const start = useRef<{ x: number; y: number } | null>(null)
  const dragged = useRef(false)
  const [offset, setOffset] = useState(0)
  return (
    <Button
      aria-label={label}
      title="上へドラッグして展開、下へドラッグして畳む"
      variant="plain" display="flex" flexDirection="column" gap="1" mx="auto" w={minimized ? '28' : '24'} h={minimized ? '12' : '8'} minW="0" p="0"
      bg={minimized ? 'bg.muted' : undefined} color="fg.muted"
      borderWidth={minimized ? '1px' : '0'} borderColor="border.emphasized" rounded="xl" shadow={minimized ? 'md' : undefined}
      _hover={minimized ? { bg: 'bg.emphasized' } : undefined}
      pointerEvents="auto" cursor="grab" _active={{ cursor: 'grabbing' }} touchAction="none" userSelect="none"
      onPointerDown={(event) => {
        if (!event.isPrimary || event.button !== 0) return
        dragged.current = false
        start.current = { x: event.clientX, y: event.clientY }
        event.currentTarget.setPointerCapture(event.pointerId)
      }}
      onPointerMove={(event) => {
        if (!start.current) return
        const distance = event.clientY - start.current.y
        if (Math.abs(distance) > 8) dragged.current = true
        setOffset(Math.max(-12, Math.min(12, distance)))
      }}
      onPointerUp={(event) => {
        if (!start.current) return
        const dy = event.clientY - start.current.y
        const dx = event.clientX - start.current.x
        start.current = null
        setOffset(0)
        if (Math.abs(dy) >= 24 && Math.abs(dy) > Math.abs(dx)) {
          dragged.current = true
          if (dy < 0) onUp()
          else onDown()
        }
      }}
      onPointerCancel={() => { start.current = null; setOffset(0); dragged.current = true }}
      onLostPointerCapture={() => { start.current = null; setOffset(0) }}
      onClick={(event) => { if (event.detail === 0 || !dragged.current) onActivate(); dragged.current = false }}
      onKeyDown={(event) => {
        if (event.key === 'ArrowUp' || event.key === 'ArrowDown') {
          event.preventDefault()
          if (event.key === 'ArrowUp') onUp()
          else onDown()
        }
      }}
    >
      <Box aria-hidden="true" w={minimized ? '8' : '12'} h="1" rounded="full" bg="fg.muted" transform={`translateY(${offset}px)`} />
      {minimized && <ChevronsUp size={16} aria-hidden="true" />}
    </Button>
  )
}

import { Switch, VisuallyHidden } from '@chakra-ui/react'
import { Moon, Sun } from 'lucide-react'
import { useTheme } from 'next-themes'

export function ColorModeSwitch() {
  const { resolvedTheme, setTheme } = useTheme()
  const dark = resolvedTheme === 'dark'

  return (
    <Switch.Root
      checked={dark}
      onCheckedChange={({ checked }) => setTheme(checked ? 'dark' : 'light')}
      size="lg"
      py="2"
      cursor="pointer"
      title="ダークモード"
    >
      <Switch.HiddenInput role="switch" />
      <Switch.Control>
        <Switch.Thumb />
        <Switch.Indicator fallback={<Sun size={14} />} color="fg.muted" _checked={{ color: 'colorPalette.contrast' }}>
          <Moon size={14} />
        </Switch.Indicator>
      </Switch.Control>
      <Switch.Label asChild><VisuallyHidden>Dark mode</VisuallyHidden></Switch.Label>
    </Switch.Root>
  )
}

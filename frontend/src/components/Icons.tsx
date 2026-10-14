import { useTheme } from 'next-themes'
import type { IconBaseProps, IconType } from 'react-icons'
import {
  LuArrowDown,
  LuArrowLeft,
  LuRefreshCw,
  LuChevronsUp,
  LuListCheck,
  LuGripVertical,
  LuListPlus,
  LuMoon,
  LuListMusic,
  LuRepeat1,
  LuRepeat,
  LuSun,
  LuTrash2,
} from 'react-icons/lu'
import { IoPlayOutline, IoPlay, IoPauseOutline, IoPause, IoReloadOutline, IoReload, IoShuffleOutline, IoShuffle, IoPlayBackOutline, IoPlayBack, IoPlayForwardOutline, IoPlayForward } from 'react-icons/io5'
import { PiVinylRecord, PiVinylRecordFill } from 'react-icons/pi'
import { RiForward15Line, RiForward15Fill, RiReplay15Line, RiReplay15Fill } from 'react-icons/ri'
import { TbArrowsShuffle, TbRepeatOff } from 'react-icons/tb'
export { SiGithub as Github } from 'react-icons/si'

function themedIcon(outline: IconType, filled: IconType = outline) {
  return function ThemeIcon({ size = 24, ...props }: IconBaseProps) {
    const { resolvedTheme } = useTheme()
    const Icon = resolvedTheme === 'dark' ? filled : outline
    return <Icon size={size} aria-hidden="true" {...props} />
  }
}

export const ArrowDown = themedIcon(LuArrowDown)
export const ArrowLeft = themedIcon(LuArrowLeft)
export const RotateCcw = themedIcon(IoReloadOutline, IoReload)
export const RefreshCw = themedIcon(LuRefreshCw)
export const ChevronsUp = themedIcon(LuChevronsUp)
export const ListCheck = themedIcon(LuListCheck)
export const GripVertical = themedIcon(LuGripVertical)
export const ListPlus = themedIcon(LuListPlus)
export const Moon = themedIcon(LuMoon)
export const Pause = themedIcon(IoPauseOutline, IoPause)
export const Play = themedIcon(IoPlayOutline, IoPlay)
export const ListMusic = themedIcon(LuListMusic)
export const Repeat = themedIcon(LuRepeat)
export const SkipBack = themedIcon(IoPlayBackOutline, IoPlayBack)
export const SkipForward = themedIcon(IoPlayForwardOutline, IoPlayForward)
export const Repeat1 = themedIcon(LuRepeat1)
export const Sun = themedIcon(LuSun)
export const Trash2 = themedIcon(LuTrash2)
export const Disc3 = themedIcon(PiVinylRecord, PiVinylRecordFill)
export const Forward15 = themedIcon(RiForward15Line, RiForward15Fill)
export const Replay15 = themedIcon(RiReplay15Line, RiReplay15Fill)
export const RepeatOff = themedIcon(TbRepeatOff)

export const Shuffle = themedIcon(IoShuffleOutline, IoShuffle)
export const ShuffleRepeat = themedIcon(TbArrowsShuffle)

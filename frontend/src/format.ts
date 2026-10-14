import type { ArtistCredit } from './api/generated'

export function artistName(credits: ArtistCredit[]): string {
  return credits.map((credit, index) => credit.name + (credit.joinPhrase ?? (index < credits.length - 1 ? '、' : ''))).join('')
}

export function formatTime(seconds: number): string {
  const total = Math.floor(Math.max(0, Number.isFinite(seconds) ? seconds : 0))
  const minutes = Math.floor(total / 60)
  return `${minutes}:${String(total % 60).padStart(2, '0')}`
}

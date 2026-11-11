import { Link } from '@chakra-ui/react'
import { Fragment } from 'react'
import { Link as RouterLink } from 'react-router-dom'
import type { ArtistCredit } from '../api/generated'

export function ArtistCredits({ credits }: { credits: ArtistCredit[] }) {
  return credits.map((credit, index) => <Fragment key={`${credit.artistId}-${index}`}>
    <Link asChild color="inherit"><RouterLink to={`/artists/${credit.artistId}`}>{credit.name}</RouterLink></Link>
    {credit.joinPhrase ?? (index < credits.length - 1 ? '、' : '')}
  </Fragment>)
}

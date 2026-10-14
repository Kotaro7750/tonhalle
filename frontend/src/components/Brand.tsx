import { Image } from '@chakra-ui/react'
import { useTheme } from 'next-themes'
import { useEffect } from 'react'
import lightLogo from '../../../logo/tonhalle-logo-with-icon.svg'
import darkLogo from '../../../logo/tonhalle-logo-with-icon-dark.svg'

export function Brand() {
  const { resolvedTheme } = useTheme()
  const dark = resolvedTheme === 'dark'

  useEffect(() => {
    const favicon = document.querySelector<HTMLLinkElement>('link[rel="icon"]')
    if (favicon) favicon.href = `${import.meta.env.BASE_URL}${dark ? 'favicon-dark.png' : 'favicon.png'}`
  }, [dark])

  return <Image src={dark ? darkLogo : lightLogo} alt="tonhalle" height="10" width="auto" />
}

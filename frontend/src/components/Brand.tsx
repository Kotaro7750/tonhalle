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
    const themeColor = document.querySelector<HTMLMetaElement>('meta[name="theme-color"]')
    // next-themes applies its class in an effect; read the resolved token after that update.
    const frame = requestAnimationFrame(() => {
      if (themeColor) themeColor.content = getComputedStyle(document.body).backgroundColor
    })
    return () => cancelAnimationFrame(frame)
  }, [dark])

  return <Image src={dark ? darkLogo : lightLogo} alt="tonhalle" height="10" width="auto" />
}

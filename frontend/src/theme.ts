import { createSystem, defaultConfig, defineConfig } from '@chakra-ui/react'

const sansFont = '"Helvetica Neue", Arial, "Hiragino Kaku Gothic ProN", "Yu Gothic", "Noto Sans JP", sans-serif'

export const system = createSystem(defaultConfig, defineConfig({
  globalCss: {
    body: { bg: 'bg', color: 'fg' },
    'html': { colorPalette: 'teal' },
  },
  theme: {
    tokens: {
      fonts: {
        heading: { value: sansFont },
        body: { value: sansFont },
      },
    },
    recipes: {
      heading: { base: { fontWeight: 'bold', letterSpacing: 'tight' } },
    },
  },
}))

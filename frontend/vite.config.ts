import react from '@vitejs/plugin-react'
import { defineConfig, loadEnv } from 'vite'

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), '')
  return {
    plugins: [react()],
    resolve: { tsconfigPaths: true },
    server: {
      proxy: { '/api': { target: env.API_PROXY_TARGET || 'http://localhost:3000', changeOrigin: true } },
    },
  }
})

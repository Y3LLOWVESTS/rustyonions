import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  server: {
    port: 5173,
    proxy: {
      '/api': 'http://127.0.0.1:5300',
      '/healthz': 'http://127.0.0.1:5310',
      '/readyz': 'http://127.0.0.1:5310',
      '/metrics': 'http://127.0.0.1:5310'
    }
  }
})

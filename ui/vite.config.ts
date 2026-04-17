import { sveltekit } from '@sveltejs/kit/vite';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [tailwindcss(), sveltekit()],
  server: {
    proxy: {
      '/api': 'http://127.0.0.1:8080',
      '/auth': 'http://127.0.0.1:8080',
      '/ingest': 'http://127.0.0.1:8080',
      '/setup': 'http://127.0.0.1:8080',
      '/health': 'http://127.0.0.1:8080'
    }
  }
});

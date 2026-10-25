import { defineConfig } from 'vite';
import { fileURLToPath } from 'node:url';

export default defineConfig({
  root: fileURLToPath(new URL('.', import.meta.url)),
  build: { outDir: 'dist', emptyOutDir: true },
  publicDir: 'public',
  server: { proxy: { '/data': 'http://127.0.0.1:8787' } },
});

import path from 'node:path';

import react from '@vitejs/plugin-react';
import { loadEnv } from 'vite';
import { defineConfig } from 'vitest/config';
import { resolveManualChunk } from './src/build/manualChunks';

const normalizeBase = (value: string) => {
  if (!value || value === '/') {
    return '/';
  }
  const withLeading = value.startsWith('/') ? value : `/${value}`;
  return withLeading.endsWith('/') ? withLeading : `${withLeading}/`;
};

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, __dirname, 'VITE_');
  const rawBase = env.VITE_ROUTER_BASENAME || env.VITE_BASE_URL || '/';
  const base = normalizeBase(rawBase);

  return {
    base,
    plugins: [react()],
    resolve: {
      alias: {
        '@': path.resolve(__dirname, 'src'),
      },
    },
    test: {
      environment: 'node',
      include: ['src/**/*.test.ts'],
    },
    server: {
      port: 5173,
      strictPort: true,
    },
    build: {
      sourcemap: true,
      rollupOptions: {
        output: {
          manualChunks: resolveManualChunk,
        },
      },
    },
  };
});

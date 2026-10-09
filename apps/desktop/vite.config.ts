import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  css: { postcss: { plugins: [] } },
  clearScreen: false,
  test: { environment: 'jsdom', setupFiles: './src/test-setup.ts', maxWorkers: 2 },
});

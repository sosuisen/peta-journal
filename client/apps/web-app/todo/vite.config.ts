import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  // Relative URLs, as the journal server serves the app under a folder like /apps/todo/
  // (ADR 073)
  base: './',
  plugins: [react()],
  server: {
    port: 5180,
  },
});

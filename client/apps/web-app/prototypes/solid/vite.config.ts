import solid from '@solidjs/vite-plugin';
import { defineConfig } from 'vite';

// The journal server of the prototype: a throwaway one with generated notes (#971)
const server = `http://127.0.0.1:${process.env.PROTO_SERVER_PORT ?? '18971'}`;

export default defineConfig({
  plugins: [solid()],
  server: {
    port: 5182,
    strictPort: true,
    proxy: { '/api': server, '/assets': server },
    fs: { allow: ['..'] },
  },
});

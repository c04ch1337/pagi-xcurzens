import path from 'path';
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

// Sovereign Monolith: No frontend .env files.
// All configuration is served by the Rust gateway.
export default defineConfig({
  server: {
    port: 3000,
    host: '0.0.0.0',
  },
  plugins: [react()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, '.'),
    }
  }
});

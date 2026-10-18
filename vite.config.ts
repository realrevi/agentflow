import { defineConfig } from 'vitest/config';
import solid from 'vite-plugin-solid';
import pkg from './package.json' with { type: 'json' };

// Tauri sets these when it runs `beforeDevCommand` / `beforeBuildCommand`.
const host = process.env.TAURI_DEV_HOST;
const isDebug = Boolean(process.env.TAURI_ENV_DEBUG);
const isWindows = process.env.TAURI_ENV_PLATFORM === 'windows';
// Component tests (jsdom) need Solid's browser build and no hot-reload wrapper.
const isTest = Boolean(process.env.VITEST);

export default defineConfig({
  plugins: [solid({ hot: !isTest })],
  resolve: isTest ? { conditions: ['browser', 'development'] } : {},
  define: {
    __APP_VERSION__: JSON.stringify(pkg.version),
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host ?? false,
    hmr: host ? { protocol: 'ws', host, port: 1421 } : undefined,
    watch: { ignored: ['**/src-tauri/**'] },
  },
  envPrefix: ['VITE_', 'TAURI_ENV_'],
  build: {
    // WebView2 on Windows is Chromium; macOS/Linux use WebKit.
    target: isWindows ? 'chrome111' : 'safari16',
    minify: !isDebug,
    sourcemap: isDebug,
  },
  test: {
    environment: 'node',
    include: ['src/**/*.test.{ts,tsx}', 'scripts/**/*.test.mjs'],
  },
});

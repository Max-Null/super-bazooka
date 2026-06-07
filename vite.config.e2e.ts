import { defineConfig, type Plugin } from "vite";
import vue from "@vitejs/plugin-vue";
import tailwindcss from "@tailwindcss/vite";
import { resolve } from "path";

// E2E test config: swaps Tauri APIs for browser-compatible mocks
// Uses a transform plugin to rewrite imports at the source level,
// which works regardless of Vite's pre-bundling.
function tauriMockPlugin(): Plugin {
  // Use forward slashes for Windows compatibility in import strings
  const MOCK_PATH = resolve(__dirname, "src/lib/tauri-mock.ts").replace(/\\/g, "/");
  return {
    name: "tauri-mock",
    enforce: "pre",
    resolveId(id) {
      if (id === "@tauri-apps/api/core" || id === "@tauri-apps/api/event") {
        return resolve(__dirname, "src/lib/tauri-mock.ts");
      }
      return null;
    },
    transform(code, id) {
      if (id.includes("node_modules")) return null;
      const rewritten = code.replace(
        /from\s+["']@tauri-apps\/api\/(core|event)["']/g,
        `from "${MOCK_PATH}"`
      );
      if (rewritten !== code) {
        return { code: rewritten, map: null };
      }
      return null;
    },
  };
}

export default defineConfig({
  plugins: [tauriMockPlugin(), vue(), tailwindcss()],
  resolve: {
    alias: {
      "@": resolve(__dirname, "./src"),
      "@tauri-apps/api/core": resolve(__dirname, "./src/lib/tauri-mock.ts"),
      "@tauri-apps/api/event": resolve(__dirname, "./src/lib/tauri-mock.ts"),
    },
  },
  server: {
    port: 1421,
    strictPort: true,
    hmr: false,
  },
  envPrefix: ["VITE_"],
  define: {
    "__TAURI_INTERNALS__": "{}",
  },
});

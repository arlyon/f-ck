import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";
import topLevelAwait from "vite-plugin-top-level-await";
import wasm from "vite-plugin-wasm";
import tsconfigPaths from "vite-tsconfig-paths";

export default defineConfig({
	plugins: [react(), wasm(), topLevelAwait(), tsconfigPaths(), tailwindcss()],
	server: {
		fs: {
			allow: [".."],
		},
	},
});

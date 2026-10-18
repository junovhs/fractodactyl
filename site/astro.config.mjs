// @ts-check
import { defineConfig } from "astro/config";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { buildStatic } from "dopedocs/static";
import { docs } from "./src/docs.ts";

const require = createRequire(import.meta.url);

/**
 * Writes the dopedocs pages (/docs/<id>/, sitemap, robots, llms.txt, JSON-LD) into the
 * finished build, once, as dopedocs' unreleased `dopedocs/astro` integration does.
 * Swap for `import dopedocs from "dopedocs/astro"` once a release ships it.
 */
function docsPages() {
  return {
    name: "fractodactyl-dopedocs",
    hooks: {
      "astro:build:done": ({ dir }) => {
        const out = fileURLToPath(dir);
        const files = buildStatic(docs, {
          entityType: "SoftwareApplication",
          stylesheet: ["/dopedocs.css", "/docs-theme.css"],
        });
        for (const [path, contents] of Object.entries(files)) {
          const file = join(out, path);
          mkdirSync(dirname(file), { recursive: true });
          writeFileSync(file, contents);
        }
        writeFileSync(join(out, "dopedocs.css"), readFileSync(require.resolve("dopedocs/styles.css")));
        console.log(`dopedocs: wrote ${Object.keys(files).length} files`);
      },
    },
  };
}

export default defineConfig({
  site: "https://fractodactyl.dev",
  integrations: [docsPages()],
});

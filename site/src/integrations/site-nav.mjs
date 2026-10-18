// DEC-31 rule 2: dopedocs has no header slot, so after the build this puts the site nav
// at the top of every generated page that lacks it (the /docs/ pages). Astro's own
// pages already render it through components/Nav.astro. Must run after dopedocs.
import { readdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { navHtml } from "../nav.ts";

async function* htmlFiles(dir) {
  for (const entry of await readdir(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) yield* htmlFiles(path);
    else if (entry.name.endsWith(".html")) yield path;
  }
}

export default function siteNav() {
  return {
    name: "fractodactyl-site-nav",
    hooks: {
      "astro:build:done": async ({ dir, logger }) => {
        const root = fileURLToPath(dir);
        let added = 0;
        for await (const file of htmlFiles(root)) {
          const html = await readFile(file, "utf8");
          if (html.includes('class="fd-sitenav"')) continue;
          const at = html.search(/<body[^>]*>/);
          if (at < 0) continue;
          const end = html.indexOf(">", at) + 1;
          const current = file.slice(root.length).startsWith("docs/") ? "/docs/" : "";
          await writeFile(file, html.slice(0, end) + navHtml(current) + html.slice(end));
          added++;
        }
        logger.info(`site nav added to ${added} generated page(s)`);
      },
    },
  };
}

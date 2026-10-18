// @ts-check
import { defineConfig } from "astro/config";
import dopedocs from "dopedocs/astro";
import { docs } from "./src/docs.ts";
import siteNav from "./src/integrations/site-nav.mjs";

export default defineConfig({
  site: "https://fractodactyl.dev",
  integrations: [dopedocs({
    docs,
    entityType: "SoftwareApplication",
    stylesheet: [true, "/docs-theme.css"],
    search: true,
  }), siteNav()],
});

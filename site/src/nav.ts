// DEC-31 rule 2: every page of fractodactyl.dev carries this one nav. Astro pages render
// it through components/Nav.astro; dopedocs pages get the same markup injected after
// the build (integrations/site-nav.mjs). check-nav.sh verifies every built page.
export const SITE_LINKS: ReadonlyArray<readonly [href: string, label: string]> = [
  ["/", "Home"],
  ["/docs/", "Docs"],
  ["/explore/", "Explore"],
  ["/download/", "Download"],
  ["https://github.com/junovhs/fractodactyl", "GitHub"],
];

/** The nav's HTML; `current` (a SITE_LINKS href) is marked aria-current. */
export function navHtml(current: string): string {
  const links = SITE_LINKS.map(([href, label]) => {
    const external = href.startsWith("http");
    const attrs = [
      `href="${href}"`,
      href === current ? `aria-current="page"` : "",
      external ? `rel="noopener"` : "",
    ].filter(Boolean).join(" ");
    return `<a ${attrs}>${label}${external ? " ↗" : ""}</a>`;
  }).join("");
  return `<nav class="fd-sitenav" aria-label="Site"><span class="fd-sitenav-brand"><img src="/favicon.png" alt="" width="20" height="20" />Fractodactyl</span><div class="fd-sitenav-links">${links}</div></nav>`;
}

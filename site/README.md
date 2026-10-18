# fractodactyl.dev

The project website: an Astro static site with the documentation written as
[dopedocs](https://github.com/junovhs/dopedocs) data.

- `src/pages/index.astro`: the home page (static HTML and CSS, no client script).
- `src/docs.ts`: the docs. Each page has a question and a self-contained answer;
  numbers live once in the facts registry. dopedocs renders them to `/docs/<id>/`,
  plus `sitemap.xml`, `robots.txt`, `llms.txt`, `questions.json` and `facts.json`.
- `public/docs-theme.css`: the palette, aliased onto dopedocs' `--dd-*` tokens.
- `public/img/`: stills rendered by `fd` (`--unresolved interior`).

Use Node 24 (`nvm use`). Then:

    npm install
    npm run dev       # local preview
    npm run deploy    # recovery only: build and publish by hand (Cloudflare, wrangler)
    ./check.sh        # the live site answers on /, /docs/what-it-is/ and /docs/
    ./check-live.sh   # the live site serves this checkout's commit (/version.txt)

Deploys are automatic (DEC-31): Cloudflare Workers Builds for `fractodactyl-site` is
connected to this repository (branch `main`, root directory `site/`). Every push to
`main` runs `npm run build && git rev-parse HEAD > dist/version.txt` and
`npx wrangler deploy` on Cloudflare; `./check-live.sh` confirms the live site serves a
given commit. Build settings live in the Cloudflare dashboard (Workers & Pages →
fractodactyl-site → Settings → Builds). Hand deploys are for recovery only.

## Documentation package

The site currently uses the committed `vendor/dopedocs-0.5.1.tgz` archive from
the tested dopedocs patch commit. `npm ci` installs it like a normal package;
there is no sibling dependency. The native Astro integration emits the bundled
stylesheet at `/docs/dopedocs.css`, then loads `/docs-theme.css` for the host
palette. Static documentation search is enabled. The temporary `buildStatic`
hook is gone.

After dopedocs 0.5.1 reaches npm, switch from this directory with:

```sh
npm install --save-exact dopedocs@0.5.1
```

Commit both package files. The integration, theme and content stay the same.
[Archive provenance and regeneration](./vendor/README.md) includes the exact
source commit and checksum, plus the command to restore the local archive.

After building, `npm run check:docs` verifies the generated documentation and
serves it locally for HTTP checks. It also runs keyboard search/navigation,
mobile and no-JavaScript browser checks when Playwright is available (set
`PLAYWRIGHT_MODULE` to an installed Playwright module if it is outside this
project). `./check.sh` can target a preview using `BASE=http://127.0.0.1:PORT`.

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

To verify an automatic deployment, run from the repository root after a push:

```sh
EXPECT=$(git rev-parse origin/main) RETRY_S=600 site/check-live.sh
```

Confirm that Cloudflare's deployment history shows a Git-triggered build for that
commit. A successful recovery hand deploy also passes the live check, so it does
not prove the Git connection works. If Builds reports a disconnected Git account,
refresh the existing Cloudflare Workers and Pages GitHub installation and reconnect
the repository, preserving the production settings above and disabling previews.

## Documentation package

Release publication calls `.github/workflows/refresh-site.yml` after every package
and `SHA256SUMS` is uploaded. It posts to the Cloudflare Deploy Hook restricted to
`fractodactyl-site`'s `main` branch, then waits for a new `/release.json` build ID
and verifies the live download links and checksums. Failures fail the workflow;
they do not silently skip the refresh. This workflow can also be dispatched for
the latest stable release without changing its version or republishing packages.

One-time setup: in Cloudflare → fractodactyl-site → Settings → Builds → Deploy
Hooks, create `github-release` for `main`. Store its URL as the GitHub repository
Actions secret `CLOUDFLARE_SITE_DEPLOY_HOOK`. The URL permits builds of that branch;
keep it secret. It requires no Cloudflare API token in GitHub. Rotate it by creating
a replacement hook, updating the secret and retiring the old hook.

`/download/` reads the latest stable GitHub Release and its `SHA256SUMS` at build
time; `/changelog/` renders the repository's `CHANGELOG.md`. A missing release
shows the source-build instructions. GitHub errors or incomplete packages fail the
build so an existing download page is not replaced with broken links. `GITHUB_TOKEN`
is optional for a higher GitHub API limit; no token is needed for public releases.
`./check-download.sh` builds offline fixtures (published, absent and incomplete
releases) and checks desktop/mobile behavior when `PLAYWRIGHT_MODULE` is set.
`./check-live.sh` also compares the live downloads and changelog to GitHub's release.

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

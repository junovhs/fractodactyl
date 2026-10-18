# fractodactyl.dev

The project website: an Astro static site with the documentation written as
[dopedocs](https://github.com/junovhs/dopedocs) data.

- `src/pages/index.astro`: the home page (static HTML and CSS, no client script).
- `src/docs.ts`: the docs. Each page has a question and a self-contained answer;
  numbers live once in the facts registry. dopedocs renders them to `/docs/<id>/`,
  plus `sitemap.xml`, `robots.txt`, `llms.txt` and `questions.json`.
- `public/docs-theme.css`: the palette, aliased onto dopedocs' `--dd-*` tokens.
- `public/img/`: stills rendered by `fd` (`--unresolved interior`).

Use Node 24 (`nvm use`). Then:

    npm install
    npm run dev       # local preview
    npm run deploy    # build and publish to fractodactyl.dev (Cloudflare, wrangler)
    ./check.sh        # the live site answers on /, /docs/what-it-is/ and /docs/

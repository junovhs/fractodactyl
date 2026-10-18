import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { createServer } from 'node:http';
import { readFile, stat } from 'node:fs/promises';
import { dirname, extname, join, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { tmpdir } from 'node:os';
import { researchPages } from '../src/research.ts';

const site = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const dist = join(site, 'dist');
const pkg = JSON.parse(await readFile(join(site, 'node_modules/dopedocs/package.json'), 'utf8'));
assert.equal(pkg.version, '0.5.1');
const manifest = JSON.parse(await readFile(join(site, 'package.json'), 'utf8'));
const dependency = manifest.dependencies.dopedocs;
if (dependency.startsWith('file:')) {
  const lock = JSON.parse(await readFile(join(site, 'package-lock.json'), 'utf8'));
  const archive = await readFile(join(site, dependency.slice(5)));
  assert.equal(`sha512-${createHash('sha512').update(archive).digest('base64')}`, lock.packages['node_modules/dopedocs'].integrity);
}
const config = await readFile(join(site, 'astro.config.mjs'), 'utf8');
assert(config.includes('from "dopedocs/astro"'));
assert(!config.includes('buildStatic') && !config.includes('docsPages'));
const html = await readFile(join(dist, 'docs/what-it-is/index.html'), 'utf8');
assert(html.includes('href="/docs/dopedocs.css"'));
assert(html.includes('href="/docs-theme.css"'));
assert(html.includes('src="/docs/search.js"'));
assert(html.includes('aria-current="page"'));
assert(html.includes('Manual reading progress'));
const index = await readFile(join(dist, 'docs/index.html'), 'utf8');
assert(!index.includes('class="dd-contents"'));
assert(!index.includes('<h2>Contents</h2>'));
assert.equal((index.match(/<nav class="dd-nav" aria-label="Contents">/g) ?? []).length, 1);
assert(index.includes('id="what-it-is"') && index.includes('id="zones"'));
const method = await readFile(join(dist, 'docs/how-we-work/index.html'), 'utf8');
assert(/<a class="dd-link"[^>]*href="https:\/\/github.com\/junovhs\/fractodactyl\/blob\/main\/docs\/spec\/METHOD.md"/.test(method));
const roadmap = await readFile(join(dist, 'docs/roadmap/index.html'), 'utf8');
assert(roadmap.includes('href="https://github.com/junovhs/fractodactyl/blob/main/docs/spec/STATE.md"'));
for (const name of ['docs/dopedocs.css', 'docs/search.js', 'sitemap.xml', 'robots.txt', 'llms.txt', 'questions.json', 'facts.json']) assert((await stat(join(dist, name))).size > 0, name);
const facts = JSON.parse(await readFile(join(dist, 'facts.json'), 'utf8'));
assert.equal(facts.facts.deep1000Frame.value, '0.023 s');
assert(!JSON.stringify(facts).includes('PROB-19'));

const mime = { '.html':'text/html; charset=utf-8', '.js':'text/javascript; charset=utf-8', '.css':'text/css; charset=utf-8', '.json':'application/json', '.svg':'image/svg+xml', '.png':'image/png', '.webp':'image/webp', '.woff2':'font/woff2' };
const server = createServer(async (request, response) => {
  try {
    let path = resolve(dist, '.' + decodeURIComponent(new URL(request.url, 'http://localhost').pathname));
    if (path !== dist && !path.startsWith(dist + sep)) throw new Error('outside root');
    if ((await stat(path)).isDirectory()) path = join(path, 'index.html');
    response.writeHead(200, { 'content-type': mime[extname(path)] ?? 'text/plain' });
    response.end(await readFile(path));
  } catch { response.writeHead(404); response.end('Not found'); }
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const base = `http://127.0.0.1:${server.address().port}`;
let browser;
try {
  for (const [path, content] of [['/', 'Fractodactyl'], ['/docs/', 'Fractodactyl user manual'], ['/docs/what-it-is/', 'What is Fractodactyl?']]) {
    const response = await fetch(base + path); assert.equal(response.status, 200); assert((await response.text()).includes(content));
  }
  // Audit the actual generated scientific reference, including its source trail.
  // Keep this tied to published routes and valid links rather than prose length.
  const researchHtml = new Map();
  const sourcePaths = new Set();
  for (const entry of researchPages) {
    const response = await fetch(`${base}/docs/${entry.id}/`);
    assert.equal(response.status, 200, entry.id);
    const body = await response.text();
    const heading = body.match(/<h1\b[^>]*>([\s\S]*?)<\/h1>/)?.[1];
    assert.equal(heading, entry.title, `missing title: ${entry.id}`);
    researchHtml.set(entry.id, body);
    for (const block of entry.blocks) {
      if (block.kind !== 'p') continue;
      for (const match of block.text.matchAll(/https:\/\/github\.com\/junovhs\/fractodactyl\/(?:blob|tree)\/main\/([^\s)]+)/g)) {
        sourcePaths.add(match[1]);
      }
    }
  }
  for (const path of sourcePaths) await stat(resolve(site, '..', path));
  for (const [id, body] of researchHtml) {
    for (const match of body.matchAll(/href="(\/docs\/[^"#?]+\/)"/g)) {
      const target = resolve(dist, '.' + match[1], 'index.html');
      assert((await stat(target)).isFile(), `broken docs link on ${id}: ${match[1]}`);
    }
  }
  console.log(`PASS research reference: ${researchPages.length} published pages, internal destinations and ${sourcePaths.size} implementation/evidence paths`);
  const response = await fetch(base + '/docs/search.js'); assert(response.headers.get('content-type').startsWith('text/javascript'));
  let playwright;
  try { playwright = await import(process.env.PLAYWRIGHT_MODULE ?? 'playwright'); }
  catch (error) { if (process.env.PLAYWRIGHT_MODULE) throw error; }
  if (playwright) {
    browser = await playwright.chromium.launch({ headless: true });
    const context = await browser.newContext();
    // External fonts are unrelated to this static integration check.
    await context.route('https://fonts.googleapis.com/**', route => route.fulfill({ contentType: 'text/css', body: '' }));
    const page = await context.newPage();
    const errors = []; const failed = [];
    page.on('pageerror', error => errors.push(error.message));
    page.on('response', response => { if (response.url().startsWith(base) && response.status() >= 400) failed.push(`${response.status()} ${response.url()}`); });
    await page.goto(base + '/docs/what-it-is/');
    await page.getByRole('searchbox').fill('7.8');
    await page.locator('.dd-search-results a').first().waitFor();
    await page.getByRole('searchbox').press('ArrowDown');
    await page.keyboard.press('Enter');
    await page.waitForURL('**/docs/zones/');
    assert.equal(await page.locator('h1').innerText(), 'Zones');
    await page.setViewportSize({ width: 390, height: 844 });
    assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
    await page.goto(base + '/docs/how-we-work/');
    assert.equal(await page.locator('a.dd-link[href$="/METHOD.md"]').count(), 1);
    // Long equations, binary-layout tables and source paths must remain readable
    // on every research page at mobile width without widening the document.
    for (const entry of researchPages) {
      await page.goto(`${base}/docs/${entry.id}/`);
      assert.equal(await page.locator('h1').innerText(), entry.title);
      const overflow = await page.evaluate(() => {
        if (document.documentElement.scrollWidth <= innerWidth) return null;
        const elements = [...document.querySelectorAll('body *')]
          .filter(el => el.getBoundingClientRect().right > innerWidth + 1 || (el.scrollWidth > el.clientWidth + 1 && getComputedStyle(el).overflowX === 'visible'))
          .map(el => ({ tag: el.tagName, class: el.className, text: el.textContent.slice(0, 100), right: el.getBoundingClientRect().right, scroll: el.scrollWidth, client: el.clientWidth }))
          .slice(0, 12);
        return { width: document.documentElement.scrollWidth, viewport: innerWidth, elements };
      });
      if (overflow) await page.screenshot({ path: join(tmpdir(), 'fractodactyl-research-overflow.png'), fullPage: true });
      assert.equal(overflow, null, `mobile overflow: ${entry.id}: ${JSON.stringify(overflow)}`);
    }
    await page.goto(base + '/docs/research-map/');
    await page.getByRole('link', { name: 'BLA', exact: true }).first().click();
    await page.waitForURL('**/docs/bla/');
    console.log('PASS research browser: every page at mobile width, systems-map navigation to BLA');
    await page.setViewportSize({ width: 1440, height: 1000 });
    await page.goto(base + '/docs/koenigs-charts/');
    const desktopShot = join(tmpdir(), 'fractodactyl-research-desktop.png');
    await page.screenshot({ path: desktopShot });
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto(base + '/docs/sample-format/');
    const mobileShot = join(tmpdir(), 'fractodactyl-research-mobile.png');
    await page.screenshot({ path: mobileShot });
    console.log(`Research visual artifacts: ${desktopShot} ${mobileShot}`);
    assert.deepEqual(errors, []); assert.deepEqual(failed, []);
    const nojs = await browser.newContext({ javaScriptEnabled: false });
    const plain = await nojs.newPage(); await plain.goto(base + '/docs/what-it-is/');
    assert((await plain.locator('h1').innerText()).includes('Fractodactyl'));
    await plain.locator('a[rel="next"]').click(); await plain.waitForURL('**/docs/explorer/');
    console.log('PASS browser: keyboard search → zones, canonical destination, mobile width, inline source link, zero local HTTP/JS errors, no-script next navigation');
  } else console.log('NOTE: Playwright unavailable; browser checks skipped (HTTP/artifact checks only)');
  console.log(`PASS dopedocs ${pkg.version}: archive/lock integrity, native Astro integration, styled pages, inline links, search, machine-readable files and local HTTP routes`);
} finally { await browser?.close(); await new Promise(resolve => server.close(resolve)); }

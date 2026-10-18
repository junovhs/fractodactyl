import assert from 'node:assert/strict';
import { readFile, writeFile, mkdtemp, rm, stat } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve, extname } from 'node:path';
import { createServer } from 'node:http';
import { spawnSync } from 'node:child_process';

const fixturePath = resolve('tests/fixtures/release.json');
const fixture = JSON.parse(await readFile(fixturePath, 'utf8'));
const temporary = await mkdtemp(join(tmpdir(), 'fd-download-check-'));
let server, browser;
function build(fixtureFile, out, succeeds = true) {
  const result = spawnSync('npm', ['run', 'build', '--', '--outDir', out], {
    env: { ...process.env, FD_RELEASE_FIXTURE: fixtureFile }, encoding: 'utf8',
  });
  if (succeeds && result.status !== 0) throw new Error(result.stdout + result.stderr);
  assert.equal(result.status === 0, succeeds, result.stdout + result.stderr);
}
try {
  const out = join(temporary, 'release');
  build(fixturePath, out);
  const html = await readFile(join(out, 'download/index.html'), 'utf8');
  const manifest = JSON.parse(await readFile(join(out, 'release.json'), 'utf8'));
  assert.equal(manifest.tag, 'v9.8.7');
  assert.ok(manifest.buildId && manifest.commit);
  assert.equal(manifest.packages.length, 2);
  assert.ok(html.includes('data-release-tag="v9.8.7"'));
  assert.ok(html.includes('2026-10-18'));
  for (const asset of fixture.release.assets) assert.ok(html.includes(asset.browser_download_url), asset.name);
  for (const sum of ['a'.repeat(64), 'b'.repeat(64)]) assert.ok(html.includes(sum));
  for (const size of ['15.0 MiB', '6.5 MiB', '15,728,640', '6,815,744']) assert.ok(html.includes(size), size);
  for (const text of ['WebView2', 'WebKitGTK', 'Get-FileHash', 'sha256sum', 'Fixture explorer']) assert.ok(html.includes(text), text);
  assert.ok(!html.includes('<script>') && !html.includes('javascript:'));
  const home = await readFile(join(out, 'index.html'), 'utf8');
  assert.match(home, /href="\/download\/"[^>]*>.*?Download the desktop app/s);
  const changelog = await readFile(join(out, 'changelog/index.html'), 'utf8');
  assert.ok(changelog.includes('[Unreleased]') && changelog.includes('[0.5.0]'));
  for (const href of ['/docs/explorer/', '/docs/first-film/', '/docs/build/']) {
    assert.ok(html.includes(`href="${href}"`));
    assert.ok((await readFile(join(out, href, 'index.html'), 'utf8')).length > 100);
  }

  if (process.env.PLAYWRIGHT_MODULE) {
    server = createServer(async (req, res) => {
      try {
        let file = join(out, new URL(req.url, 'http://localhost').pathname);
        if ((await stat(file)).isDirectory()) file = join(file, 'index.html');
        const types = { '.html': 'text/html', '.css': 'text/css', '.js': 'text/javascript', '.png': 'image/png' };
        res.setHeader('Content-Type', types[extname(file)] ?? 'application/octet-stream');
        res.end(await readFile(file));
      } catch { res.statusCode = 404; res.end('Not found'); }
    });
    await new Promise(done => server.listen(0, '127.0.0.1', done));
    const origin = `http://127.0.0.1:${server.address().port}`;
    const { chromium } = await import(process.env.PLAYWRIGHT_MODULE);
    browser = await chromium.launch({ headless: true });
    const page = await browser.newPage();
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    await page.route('**/*', route => route.request().url().startsWith(origin) ? route.continue() : route.abort());
    for (const width of [1280, 390]) {
      await page.setViewportSize({ width, height: 960 });
      await page.goto(`${origin}/download/`);
      assert.equal(await page.getByRole('link', { name: /Download for/ }).count(), 2);
      for (const details of await page.locator('details').all()) await details.locator('summary').click();
      assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), `horizontal overflow at ${width}`);
      await page.screenshot({ path: `/tmp/fractodactyl-download-${width}.png`, fullPage: true });
      await page.getByRole('link', { name: 'Full changelog', exact: true }).click();
      assert.equal(await page.getByRole('heading', { name: 'Changelog', exact: true }).count(), 1);
    }
    assert.deepEqual(errors, []);
    console.log('PASS download browser: desktop/mobile, checksums, changelog navigation, no overflow or JS errors');
  }
  const empty = join(temporary, 'empty.json');
  await writeFile(empty, JSON.stringify({ release: null }));
  build(empty, join(temporary, 'empty'));
  const emptyHTML = await readFile(join(temporary, 'empty/download/index.html'), 'utf8');
  assert.ok(emptyHTML.includes('Packages are coming soon') && emptyHTML.includes('/docs/build/'));
  assert.ok(!emptyHTML.includes('data-download='));
  const broken = join(temporary, 'broken.json');
  await writeFile(broken, JSON.stringify({ ...fixture, checksums: fixture.checksums.split('\n')[0] }));
  build(broken, join(temporary, 'broken'), false);
  console.log('PASS downloads: release assets/date/sizes/checksums/notes, safe Markdown, home/manual links, changelog, no-release fallback; incomplete release refuses to build');
} finally {
  await browser?.close();
  if (server) await new Promise(done => server.close(done));
  await rm(temporary, { recursive: true, force: true });
}

import assert from 'node:assert/strict';
import { getLatestRelease } from '../src/lib/releases.mjs';
const base = process.env.BASE ?? 'https://fractodactyl.dev';
const release = await getLatestRelease();
async function page(path) {
  const response = await fetch(`${base}${path}`, { signal: AbortSignal.timeout(30000), headers: { 'Cache-Control': 'no-cache' } });
  assert.equal(response.status, 200, path);
  return response.text();
}
const html = await page('/download/');
assert.ok(html.includes('fd-sitenav'));
if (release) {
  assert.ok(html.includes(`data-release-tag="${release.tag}"`), 'live download version differs from latest release');
  for (const pkg of release.packages) {
    assert.ok(html.includes(pkg.url), pkg.name);
    assert.ok(html.includes(pkg.sha256), `checksum for ${pkg.name}`);
  }
  assert.ok((await page('/changelog/')).includes(`[${release.version}]`), 'released version missing from changelog');
  console.log(`ok: ${base}/download/ and /changelog/ match ${release.tag}, both packages and SHA256SUMS`);
} else {
  assert.ok(html.includes('Packages are coming soon'));
  await page('/changelog/');
  console.log('ok: download page explains that no release is available');
}

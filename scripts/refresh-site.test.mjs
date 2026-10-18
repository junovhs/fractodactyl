import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { refreshSite } from './refresh-site.mjs';

const require = createRequire(new URL('../site/package.json', import.meta.url));
const { parse } = require('yaml');
const hookURL = 'https://api.cloudflare.com/client/v4/workers/builds/deploy_hooks/11111111-1111-1111-1111-111111111111';
const buildUUID = '22222222-2222-2222-2222-222222222222';
const release = { tag: 'v9.8.7', packages: [
  { name: 'windows.zip', url: 'https://github.com/windows.zip', sha256: 'a'.repeat(64) },
  { name: 'linux.tar.gz', url: 'https://github.com/linux.tar.gz', sha256: 'b'.repeat(64) },
] };
const fresh = { buildId: 'new', tag: release.tag, packages: release.packages, commit: 'main-commit' };
const html = `<p data-release-tag="${release.tag}">${release.packages.map(p => `${p.url} ${p.sha256}`).join(' ')}</p>`;
const json = value => new Response(JSON.stringify(value), { headers: { 'Content-Type': 'application/json' } });

test('waits for a fresh build with exact release assets and live download HTML', async () => {
  const manifests = [
    { ...fresh, buildId: 'old' },
    { ...fresh, buildId: 'old' }, // old build already has the same version
    { ...fresh, tag: 'v9.8.6' },
    { ...fresh, packages: [{ ...release.packages[0], sha256: 'wrong' }] },
    fresh,
    fresh,
  ];
  const calls = [];
  let pages = 0;
  const result = await refreshSite({ release, hookURL, sleep: async () => {}, fetchImpl: async (url, options) => {
    calls.push([url, options?.method ?? 'GET']);
    if (url === hookURL) return json({ success: true, result: { build_uuid: buildUUID } });
    if (url.includes('/release.json')) return json(manifests.shift());
    return new Response(pages++ === 0 ? '<p>stale edge cache</p>' : html);
  } });
  assert.equal(result.buildUUID, buildUUID);
  assert.equal(result.manifest.buildId, 'new');
  assert.ok(calls[0][0].includes('/release.json'));
  assert.deepEqual(calls.filter(([, method]) => method === 'POST'), [[hookURL, 'POST']]);
  assert.equal(pages, 2);
});

test('fails when Cloudflare rejects the trigger', async () => {
  await assert.rejects(refreshSite({ release, hookURL, sleep: async () => {}, fetchImpl: async (url) =>
    url === hookURL ? new Response('', { status: 403 }) : json(fresh),
  }), /trigger returned HTTP 403/);
});

test('does not accept an old build or silently time out', async () => {
  await assert.rejects(refreshSite({ release, hookURL, attempts: 2, sleep: async () => {}, fetchImpl: async (url) =>
    url === hookURL ? json({ success: true, result: { build_uuid: buildUUID } }) : json(fresh),
  }), /did not publish a fresh build/);
});

test('invalid or missing hook configuration cannot trigger a build', async () => {
  for (const value of ['', 'https://example.com/collect']) {
    await assert.rejects(refreshSite({ release, hookURL: value, fetchImpl: () => { throw new Error('must not fetch'); } }), /missing or invalid/);
  }
});

test('release publication calls the same tested refresh workflow after publishing', () => {
  const workflow = parse(readFileSync(new URL('../.github/workflows/release.yml', import.meta.url), 'utf8'));
  const refresh = parse(readFileSync(new URL('../.github/workflows/refresh-site.yml', import.meta.url), 'utf8'));
  assert.deepEqual(workflow.jobs['refresh-site'].needs, ['plan', 'publish']);
  assert.equal(workflow.jobs['refresh-site'].uses, './.github/workflows/refresh-site.yml');
  assert.equal(workflow.jobs['refresh-site'].secrets.CLOUDFLARE_SITE_DEPLOY_HOOK, '${{ secrets.CLOUDFLARE_SITE_DEPLOY_HOOK }}');
  assert.equal(workflow.jobs['refresh-site'].with.release_tag, 'v${{ needs.plan.outputs.version }}');
  assert.ok(refresh.on.workflow_call && refresh.on.workflow_dispatch);
  assert.equal(refresh.jobs.refresh.steps.at(-1).run, 'node scripts/refresh-site.mjs');
  assert.ok(!JSON.stringify(workflow.jobs.publish.steps).includes('gh workflow run site.yml'));
});

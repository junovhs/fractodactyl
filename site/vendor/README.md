# Temporary dopedocs release archive

`dopedocs-0.5.1.tgz` is the npm package built from
[`junovhs/dopedocs` commit `3a9981c`](https://github.com/junovhs/dopedocs/tree/3a9981cc3625ae86f32e632ffcf1d728a0bdafb6),
commit `3a9981cc3625ae86f32e632ffcf1d728a0bdafb6`.

It is committed here because npm publishing is awaiting the owner's trusted
publisher configuration. A clean clone and deployment build need no sibling
checkout, npm link, or source build. The site imports the normal public
`dopedocs/astro` API, so changing package sources requires no integration edit.
The package is MIT; its licence is inside the archive.

Archive integrity (also recorded in package-lock.json):

```
sha512-DWeZymRtNkgy4aiJ4+/eyNHmbEEz9RKN7m5UvJu/5mbUgaEVYCqM8JFci5Q6gF/L93qCEjWuHL2vSDl006KsDQ==
```

## Switch to npm once 0.5.1 is published

From `site/`:

```sh
npm install --save-exact dopedocs@0.5.1
npm run build
```

Commit package.json and package-lock.json together. This replaces the `file:`
dependency with the registry version; no Astro, theme or content changes are
needed. Use an explicit version, because `latest` still points to 0.4.0 until
publishing succeeds. The archive may be removed after the switch is verified.

To restore this snapshot before removing it:

```sh
npm install --save-exact ./vendor/dopedocs-0.5.1.tgz
```

## Recreate the archive

Use a clean export of the recorded dopedocs commit, then run `npm ci` and
`npm pack --ignore-scripts --pack-destination /path/to/fractodactyl/site/vendor`.
The install's prepare hook builds the distributable. Check the archive integrity
against the value above and update provenance/version/lockfile together for a
new snapshot. Do not repack a dirty sibling checkout or publish from here.

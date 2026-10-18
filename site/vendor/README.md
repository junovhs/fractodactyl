# Temporary dopedocs release archive

`dopedocs-0.5.0.tgz` is the npm package built from
[`junovhs/dopedocs` tag `v0.5.0`](https://github.com/junovhs/dopedocs/tree/v0.5.0),
commit `26f70105d5ccd11479ae23b878c140e1c30fe30d`.

It is committed here because npm publishing is awaiting the owner's trusted
publisher configuration. A clean clone and deployment build need no sibling
checkout, npm link, or source build. The site imports the normal public
`dopedocs/astro` API, so changing package sources requires no integration edit.
The package is MIT; its licence is inside the archive.

Archive integrity (also recorded in package-lock.json):

```
sha512-8pXd5YZ8zFOID46F87AF0Hl7+nG03EbGS1gQHH9HrmD1KU/6UoLkPsoCLpzF05uHU2k66ejw0YHWvf0iWHjlNg==
```

## Switch to npm once 0.5.0 is published

From `site/`:

```sh
npm install --save-exact dopedocs@0.5.0
npm run build
```

Commit package.json and package-lock.json together. This replaces the `file:`
dependency with the registry version; no Astro, theme or content changes are
needed. Use an explicit version, because `latest` still points to 0.4.0 until
publishing succeeds. The archive may be removed after the switch is verified.

To restore this snapshot before removing it:

```sh
npm install --save-exact ./vendor/dopedocs-0.5.0.tgz
```

## Recreate the archive

Use a clean export of the recorded dopedocs tag, then run `npm ci` and
`npm pack --ignore-scripts --pack-destination /path/to/fractodactyl/site/vendor`.
The install's prepare hook builds the distributable. Check the archive integrity
against the value above and update provenance/version/lockfile together for a
new snapshot. Do not repack a dirty sibling checkout or publish from here.

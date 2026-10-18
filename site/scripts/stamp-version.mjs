// SITE-08: write the deployed commit to dist/version.txt so check-live.sh can tell
// which commit fractodactyl.dev is serving (DEC-31 rule 3).
import { execFileSync } from "node:child_process";
import { writeFileSync } from "node:fs";

const sha = process.env.GITHUB_SHA
  || execFileSync("git", ["rev-parse", "HEAD"], { encoding: "utf8" }).trim();
writeFileSync(new URL("../dist/version.txt", import.meta.url), `${sha}\n`);
console.log(`dist/version.txt: ${sha}`);

#!/usr/bin/env bash
# SITE-09 check (DEC-31 rule 2): every built page carries the one site nav, and the nav
# has each of the five links exactly once. Run after `npm run build`.
set -u
DIST="${DIST:-dist}"
links=('href="/"' 'href="/docs/"' 'href="/explore/"' 'href="/download/"' 'href="https://github.com/junovhs/fractodactyl"')
fail=0; pages=0
while IFS= read -r -d '' f; do
  pages=$((pages + 1))
  navs=$(grep -o 'class="fd-sitenav"' "$f" | wc -l)
  if [ "$navs" -ne 1 ]; then echo "FAIL: $f has $navs site navs"; fail=1; continue; fi
  nav=$(grep -o '<nav class="fd-sitenav".*</nav>' "$f" | sed 's#</nav>.*#</nav>#')
  for l in "${links[@]}"; do
    n=$(grep -oF "$l" <<<"$nav" | wc -l)
    [ "$n" -eq 1 ] || { echo "FAIL: $f nav has $l $n times"; fail=1; }
  done
done < <(find "$DIST" -name '*.html' -print0)
[ "$pages" -gt 0 ] || { echo "FAIL: no pages in $DIST"; exit 1; }
[ "$fail" -eq 0 ] && echo "ok: $pages pages carry the site nav"
exit "$fail"

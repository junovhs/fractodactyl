# places/

Shipped places. Only places the owner deliberately curates, each verified to show
what it claims, are committed here or packaged into a release (DEC-35). Explorer
and viewer saves are personal user data: `.gitignore` ignores `places/*.place`, and
the release package copies only tracked places (`scripts/package-places.sh`).
To ship a curated place, add a `!places/NAME.place` negation to `.gitignore`.

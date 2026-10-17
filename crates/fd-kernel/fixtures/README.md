# Kernel fixture provenance

`KERN-05.bundle` was captured on Windows from the pre-refactor renderer at
`97c532cb8fb832fe05e529a35d678f558162ef6b`, with SHA-256
`f9c174de24ef2c7b72cf154cd1833d7e46fb239f87e97bd4789877f38f692cfb`.

CLEA-04 deleted the CPU zone, and with it the bundle's six `zone-*` entries. The
remaining twelve perturbation-kernel entries are byte-for-byte the originals, cut
out of the original file. The bundle's SHA-256 is now
`cc9108a1f3fe4362febd074b4a538bcb818042f4cd3b49855496256d28073314`. Zone frames are
checked by fd-cli's `zone_fixtures` test against frozen frames
(`crates/fd-cli/tests/fixtures/zones`).

The 8x4 zone byte-hash test (`zone_pixel_render_bytes_are_unchanged`, FIX-55) went
with the CPU zone in CLEA-04.

The measured targets were x86-64 Linux with Rust 1.91.1 and x86-64 Windows MSVC
with Rust 1.99.0.

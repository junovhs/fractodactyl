# Kernel fixture provenance

`KERN-05.bundle` was captured on Windows from the pre-refactor renderer at
`97c532cb8fb832fe05e529a35d678f558162ef6b`. Its SHA-256 is
`f9c174de24ef2c7b72cf154cd1833d7e46fb239f87e97bd4789877f38f692cfb`.

The small `zone_pixel_render_bytes_are_unchanged` test freezes four complete 8x4
frames with Nu, De and Normal columns. FIX-55 rendered these cases with both that
pre-refactor renderer and the shared renderer on each platform. All four frame
fingerprints matched within each platform; the Windows and Linux hashes
differed. The test therefore retains separate frozen values for Linux and Windows.

| Zone | Width | Linux FNV-1a | Windows FNV-1a |
| --- | --- | --- | --- |
| Ordinary | `1e-36` | `850c901dfbe62468` | `30499ad7f43278cd` |
| Ordinary | `2e-48` | `988c23c5fc7ef3e3` | `f17526a7d999103d` |
| Deep | `1e-36` | `b433623f669f6b7b` | `243149996becf305` |
| Deep | `2e-48` | `242f075f9b9f3e99` | `23b2b21cf0acb4e7` |

Ishoo verification jobs 66 (Windows) and 67 (Linux) record the baseline/current
comparisons. These values remain exact byte checks. A future fixture update must
compare with its pre-change renderer on the same platform before replacing a hash.

The measured targets were x86-64 Linux with Rust 1.91.1 and x86-64 Windows MSVC
with Rust 1.99.0. Rendering code and the existing Windows fixture bundle are unchanged.

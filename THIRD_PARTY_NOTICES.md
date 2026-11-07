# Third-party notices

EarthPress itself is MIT licensed (see `LICENSE`). It is built on the crates
below, all of which are permissively licensed. Full licence texts for the
non-MIT/Apache ones are vendored in `licenses/`; the MIT/Apache texts ship with
each crate in the local Cargo registry.

## Required acknowledgement

> This software is based in part on the work of the Independent JPEG Group.

That sentence is required by the IJG terms that cover the IJG-derived portions
of `jpeg-encoder` (its forward DCT and the AVX2 variant), and it must appear in
the accompanying documentation of any distributed executable. The full text is
in `licenses/IJG.txt`.

## Direct dependencies

| Crate | Licence | Notice text |
|-------|---------|-------------|
| `tauri` | MIT OR Apache-2.0 | ships with crate |
| `tauri-plugin-dialog` | MIT OR Apache-2.0 | ships with crate |
| `tauri-plugin-opener` | MIT OR Apache-2.0 | ships with crate |
| `tauri-plugin-notification` | MIT OR Apache-2.0 | ships with crate |
| `notify-rust`, `tauri-winrt-notification` | MIT OR Apache-2.0 | ships with crate |
| `serde`, `serde_json` | MIT OR Apache-2.0 | ships with crate |
| `image` | MIT OR Apache-2.0 | ships with crate |
| `png` | MIT OR Apache-2.0 | ships with crate |
| `zenwebp` (pinned to 0.2.x) | MIT OR Apache-2.0 | ships with crate |
| `jpeg-encoder` | (MIT OR Apache-2.0) AND IJG | `licenses/IJG.txt` |
| `ravif` | BSD-3-Clause | `licenses/ravif-BSD-3-Clause.txt` |
| `avif-decode` | BSD-3-Clause | `licenses/avif-decode-BSD-3-Clause.txt` |
| `rav1e` | BSD-2-Clause | `licenses/rav1e-BSD-2-Clause.txt` |
| `rav1d` | BSD-2-Clause | `licenses/rav1d-BSD-2-Clause.txt` |
| `kamadak-exif` | BSD-2-Clause | `licenses/kamadak-exif-BSD-2-Clause.txt` |

## Bundled fonts

EarthPress ships two typefaces as latin-subset WOFF2 files instead of calling
out to a font CDN, so the offline promise holds:

| Font | Licence | Notice text |
|------|---------|-------------|
| DM Sans | SIL Open Font License 1.1 | `licenses/DM-Sans-OFL-1.1.txt` |
| Bricolage Grotesque | SIL Open Font License 1.1 | `licenses/Bricolage-Grotesque-OFL-1.1.txt` |

Both are distributed unmodified as bundled webfonts; the fonts themselves are
not sold, and the app does not restrict their use by anyone else.

## Licence-critical constraints

**Do not bump `zenwebp` past 0.2.x.** `zenwebp` 0.2.1 is MIT OR Apache-2.0, but
0.3.0 and every later release are AGPL-3.0-or-commercial. The pin in
`src-tauri/Cargo.toml` is deliberate and load-bearing for this project's MIT
licence; bumping it would relicense the whole application.

**Do not enable the AV1 `asm` features.** `ravif`/`rav1e` (encoding) are
configured with `default-features = false`, and the `rav1d` decoder behind
`avif-decode` is patched the same way in `src-tauri/vendor/avif-decode`. Turning
either back on makes the build require NASM. All of `ravif`, `rav1e`, `rav1d` and
`avif-decode` are permissively licensed (BSD-2/BSD-3-Clause) either way, so this
is purely about avoiding an assembler dependency.

**The vendored `avif-decode` is a modified copy.** It is BSD-3-Clause; upstream's
licence text is kept at `src-tauri/vendor/avif-decode/LICENSE` and the exact
change is documented in that directory's `Cargo.toml` header and in `README.md`.

No AGPL, GPL or other copyleft dependency is present in the dependency graph.

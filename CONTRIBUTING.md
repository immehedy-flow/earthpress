# Contributing

Thanks for taking a look at EarthPress. The project is a Tauri v2 desktop app:
a React/TypeScript frontend in `src/` and a pure-Rust image pipeline in
`src-tauri/src/`.

## Before you start

Read the licence constraints in `README.md` and `THIRD_PARTY_NOTICES.md`. Two
rules are load-bearing:

* **Do not bump `zenwebp` past 0.2.x.** Later releases are AGPL-3.0.
* **Do not enable the AV1 `asm` features.** They require NASM and break the
  pure-Rust build.

## Setup

```powershell
npm install
npm run dev          # tauri dev
```

Rust 1.98+ with the MSVC toolchain and the Windows SDK are required. No C
compiler, NASM or CMake is needed.

## Checks

```powershell
npx tsc -b
npx oxlint src
cargo test --manifest-path src-tauri/Cargo.toml
```

CI runs the same three commands on Windows. Please keep the Rust suite green and
add a test when you change the pipeline, path handling or estimates.

## Pull requests

* Keep the change focused; separate UI, backend and dependency updates.
* Match the existing style (two-space TS, `rustfmt` defaults, comments explain
  *why*).
* Update `README.md` or `THIRD_PARTY_NOTICES.md` when behaviour or
  dependencies change.
* Describe the user-visible effect and how you verified it (browser harness,
  native run, or tests).

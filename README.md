# PACE adventure editor

A Dioxus 0.5.6 editor targeting **web / WebAssembly** first. Rust is pinned to
1.90.0. Desktop and the old resolver crates are retained but excluded from the
supported workspace.

## Run

```sh
rustup show # installs rust-toolchain.toml's toolchain and WASM target
cargo install trunk --version 0.21.14 --locked
trunk serve
```

Open http://localhost:8080. `trunk build --release` produces a static site in
`dist/`. No server or account is needed for project authoring.

```sh
cargo test --workspace
cargo check -p web_view --target wasm32-unknown-unknown
cargo fmt --all -- --check
```

## Core dependency — blocked

The inspected `pace_core` commit is
[`6ccfd6ea60534f3c3312882458e1c83308a76b81`](https://github.com/pace-kit/pace_core/tree/6ccfd6ea60534f3c3312882458e1c83308a76b81).
It contains empty build/debug modules and Bevy templates, but no versioned scene
format, loader, executable runtime, or three-room fixture. No other branches or
PRs were available during implementation. `runtime.lock.json` records this
fact; its null runtime revision is **not** a runnable runtime pin.

Projects currently use **`pace-editor-draft`, version 1**, an explicitly
editor-owned contract, not a claimed implementation of the missing core format.
Core save/import, runtime preview, and game export cannot be completed until
that dependency exists. Draft files are deliberately named `.pace-draft.json`.
No upstream compatibility is implied.

## Project files

The JSON envelope includes its format/version, project title, start-room ID,
stable object IDs, embedded PNG/JPEG/WebP backgrounds, normalized hotspot
rectangles, room links, items, dialogue nodes and choices. Embedded images make
files portable without absolute paths or missing asset folders. Limits: 8 MiB
per background, 16,384 pixels per dimension, 64 MiB per project file.

Unknown fields, unknown versions, duplicate IDs, and malformed files are rejected
before replacing the current project. Broken references remain editable and are
reported against their owning objects. Deleting a referenced object deliberately
leaves those references visible; undo restores it. History retains 50 edits.
Save downloads the full project and attempts an atomic browser recovery copy.
Recovery is origin-specific and limited by browser quota; a warning instructs you
to save a file if it is unavailable. Project files are the portable source of
truth. Keep a downloaded copy before clearing browser data.

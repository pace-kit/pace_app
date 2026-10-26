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

## Project sharing

Open **Project sharing** to connect to `pace_server`, upload a private snapshot,
grant reader/writer access, and download another user's project. Uploads require
the revision you downloaded; stale writes keep your edits and show a conflict.
Downloads are staged for review before replacing the open project. Tokens stay
in memory and are cleared on reload. Local save/recovery works without a server.
See [setup and the two-client walkthrough](docs/project-sharing.md).

This is **project sharing, not real-time co-editing**. There is no automatic
upload, merge, polling or live presence.

## Core and draft formats

`pace_core` at
[`1aa2b36270b68ea6fc04c268c3d5661ec09790aa`](https://github.com/pace-kit/pace_core/tree/1aa2b36270b68ea6fc04c268c3d5661ec09790aa)
now defines core scene v1 and a runnable native adventure runtime. The server
validates core scenes with that pinned loader, independently of editor drafts.
The editor does not yet convert drafts to core scenes or run the core in a browser;
`runtime.lock.json`'s null runtime revision is **not** a runnable web runtime pin.

Projects currently use **`pace-editor-draft`, version 1**, an explicitly
editor-owned contract, with embedded images and editable incomplete references.
The model and loader are shared with the server through the `pace_editor_format`
crate. Sharing does not change that schema or convert it into a core scene.
Draft files remain `.pace-draft.json`; core conversion, preview and game export
require a separate integration that can account for their different features.

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

## Room tools

Add a room and import its background. Use **Draw hotspot** to drag a rectangle
in either direction, or **Add hotspot** for keyboard-friendly placement. Drag a
selected hotspot to move it; arrow keys nudge it by 1% of the image. Set exact
X/Y/width/height percentages in the inspector. Image replacement preserves the
normalized bounds. A **Go to room** action links to a stable room ID, so renaming
a destination does not break the link. Add a separate return hotspot for a
bidirectional connection.

Validation entries identify the object ID and field; click an entry to select
its room, hotspot, item, or dialogue. Saves may contain incomplete drafts.
Invalid drafts cannot be rehearsed.

## Browser checks

```sh
npm ci
npx playwright install --with-deps chromium
trunk build
# Build the compatible server (see docs/project-sharing.md), then:
export PACE_SERVER_BIN=/absolute/path/to/pace_server/target/debug/pace_server
npm run test:e2e
```

For an existing Chromium installation, set `CHROMIUM_PATH=/usr/bin/chromium`.

## Dialogue, inventory, and walkthrough

Create inventory definitions and dialogue nodes from their sidebar sections.
Dialogue choices can branch to another node, grant an item, or end the
conversation. Hotspots can inspect text, link rooms, pick up an item, or start a
dialogue; any action can require an inventory item. Names are editable while IDs
stay stable.

Follow [the illustrated walkthrough](docs/walkthrough.md) to author **The Garden
Key**, a three-room example, using only the editor controls. The browser test
reproduces the walkthrough, including save/reopen equality for the whole project.

**Rehearse draft** runs an isolated editor simulation to exercise the draft's
links, dialogue branches and item gates. Restart clears its session. It never
modifies the source project and is **not** the PACE core runtime. **Core preview**
and **Export game** remain disabled. This implementation does not yet meet the
core-format conversion or pinned-runtime preview/export requirements. Snapshot
sharing is available for the editor format independently of runtime integration.

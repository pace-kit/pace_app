# Core integration boundary

Do not reinterpret the draft schema as an upstream PACE format. Core at
`1aa2b36270b68ea6fc04c268c3d5661ec09790aa` now supplies a versioned scene contract,
validation with JSON paths and a native runtime. Core v1 has no asset-loading
contract for the editor's embedded backgrounds. Draft conversion and a runnable
web target still need integration before runtime preview/export can be enabled.
Project sharing reuses the unchanged draft loader; it does not perform conversion.

1. Set `runtime.lock.json.runtime_revision` to the actual immutable core commit.
2. Add the core crate using `git` + `rev` (and commit Cargo.lock), or pin its WASM
   artifact with a checksum. Never use a moving branch in shipped exports.
3. Add a separate conversion adapter from `model::Project` to the core model.
   Check all conversions; report unsupported fields on their owning object.
4. Test a canonical core three-room fixture against that pinned loader, including
   assets, room transitions, conditional item actions and branching dialogue.
5. Route both preview and export through that same loader/runtime. Only then
   enable core export. Export must contain the pinned runtime and all assets.
6. Keep a versioned draft migration path; do not silently rename old files into
   the core format or discard fields the core version cannot represent.

The three-room authoring example in this repository is editor-owned. Replace or
compare it with the canonical core fixture when the latter is available.

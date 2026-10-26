# Sharing project snapshots

Keep using **Save project** and **Open project** for portable local files.
Sharing sends the same `pace-editor-draft` v1 JSON, including embedded backgrounds,
object IDs, dialogue, inventory and unfinished references. It never adds credentials,
server IDs or revisions to that document. The server stores the original upload
bytes. A later editor save uses its normal pretty JSON serializer.

## Server setup

Use a `pace_server` build with scene/draft validation and browser-origin support
([server PR #3](https://github.com/pace-kit/pace_server/pull/3), revision
`66203da9bfeda679dac0669fe4eae7ced9d9e59e` or a later compatible release).
See its README for operator token provisioning and persistent SQLite storage.
Each person receives a token mapped to a stable member ID such as `alice` or `bob`.
Run the server with the editor's exact browser origin in `PACE_ALLOWED_ORIGINS`:

```sh
PACE_AUTH_FILE=./auth.json PACE_DATABASE=./projects.sqlite3 \
PACE_ALLOWED_ORIGINS=http://127.0.0.1:8080,http://localhost:8080 cargo run --locked
```

In **Project sharing**, enter the server URL and your bearer token. HTTPS is
required except on loopback (`localhost`, `127.0.0.1`, `[::1]`). A reverse proxy
may serve the API under a URL prefix. There are no wildcard origins or cookies;
project requests use bearer authentication. Redirects are refused so credentials
cannot follow a redirected request. Tokens are kept in component memory only,
with **Clear token** and reload clearing the input. Do not paste a token into
the project itself. No server login/account creation is supplied by this editor.

## Two clients

1. Alice opens or authors a draft and chooses **Upload as new project**. Copy the
   resulting **Shared project ID**.
2. Alice enters Bob's member ID, selects Reader or Writer, and chooses **Grant
   access**. Give Bob the project ID and server URL through your usual channel.
3. Bob enters his own token, the same server URL and project ID, then chooses
   **Download project**. The current local document remains open. Choose **Open
   downloaded project** to replace it, with confirmation if there are unsaved edits.
4. Bob edits locally and chooses **Upload changes**. This sends `If-Match` with
   the revision of the snapshot he opened. Alice can explicitly download his update.
5. Use **Save project** on each client for a local backup. Uploading does not clear
   the local unsaved-file indicator. Only the owner can grant/revoke access; a writer
   cannot change permissions. Reader uploads are denied.

## Conflicts and failed requests

If both clients edit revision 1, the first successful upload creates revision 2.
The other's upload returns HTTP `412` and a conflict message. Its edits, undo
history and upload precondition remain intact; there is no automatic retry.
**Download project** stages the latest snapshot without advancing the revision
attached to local edits. Then explicitly choose one of:

- **Open downloaded project**: use the downloaded copy after saving any local work.
- **Keep local edits against downloaded revision**: after reviewing/reconciling,
  confirm that the next upload should replace that downloaded snapshot. This does
  not upload automatically. Another intervening server update will still conflict.

Network errors/timeouts leave the outcome of an upload uncertain. Keep your local
file and inspect/download the remote snapshot before retrying. Requests time out
after 30 seconds, with a 64 MiB response/upload limit. Missing/invalid tokens,
denied access, unsupported drafts and malformed responses leave the open document
unchanged. Opening a different document while a request is in flight prevents the
late result from attaching its revision to the new document. Edits made during
an upload remain open and can be uploaded explicitly afterward.

Connection details, linked revision and staged downloads are session state.
After reload or opening another local file, download/open the remote project to
establish its revision again. Keep a local copy before switching documents.

## Scope and verification

This is **project sharing, not real-time co-editing**. There is no live presence,
operation stream, background polling, automatic merge or last-writer-wins update.
The server also supports core scene v1 through its HTTP API; this editor continues
to open its own draft format. Sharing does not convert either format or enable
core preview/export.

The Playwright suite starts a real `pace_server` process with a temporary database
and test-only identities. The server revision used in CI is pinned in
`.github/workflows/editor.yml`. Locally, build the server with Rust 1.97.1+:

```sh
cargo +stable build --locked --manifest-path ../pace_server/Cargo.toml
NO_COLOR=true trunk build --locked
PACE_SERVER_BIN=../pace_server/target/debug/pace_server npm run test:e2e
```

It tests two independent browser contexts, full draft/file equality including
images, stale writes, explicit reconciliation, invalid tokens, access revocation,
malformed downloads and late responses. Existing local authoring/rehearsal checks
continue to run. Native tests cover the session transitions independently of UI.

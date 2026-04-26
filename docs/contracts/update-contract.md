# Contract — Update

> Domain: update (use case — update_checker)
> Last updated by: update spec

## Commands

| Command            | Args | Return               | Errors                                                                                                |
| ------------------ | ---- | -------------------- | ----------------------------------------------------------------------------------------------------- |
| `check_for_update` | —    | `Option<UpdateInfo>` | *(none — network/server errors are silent per R21; command returns None)*                             |
| `download_update`  | —    | `()`                 | *(none — returns immediately (R6, R7); errors emitted as `update:error` event (R23); re-invoke to retry per R24; concurrent calls silently ignored per R10)* |
| `install_update`   | —    | `()`                 | `NoUpdateReady` *(precondition guard — inferred from R13: install requires a completed download)*     |

## Shared Types

```rust
struct UpdateInfo {
    version: String,  // semantic version of available update (e.g. "1.2.3")
}

struct UpdateProgress {
    percent: u64,  // download completion 0–100 (R8)
}
```

## Events

| Event              | Payload          | Rule    |
| ------------------ | ---------------- | ------- |
| `update:available` | `UpdateInfo`     | R1, R25 |
| `update:progress`  | `UpdateProgress` | R8      |
| `update:complete`  | —                | R11     |
| `update:error`     | error string     | R23, R9 |

## Changelog

- 2026-04-26 — Added by `update` spec: check_for_update, download_update, install_update
- 2026-04-26 — Fixed: NoUpdateReady rule citation corrected; UpdateProgress type added; R24/R10 noted on download_update; R9 added to update:error

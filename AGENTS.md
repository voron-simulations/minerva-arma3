# AGENTS.md

Thin arma-rs + SQF wrapper around [`minerva-server`](https://github.com/voron-simulations/minerva-server):
pushes Arma state into its `StateCache` and executes commands it dispatches back.

## Layout

- `src/lib.rs` — the `#[arma] fn init()` entry point and the command handlers arma-rs
  routes SQF `callExtension` calls to (`start`, `stop`, `reset`, `sim:info`, `sim:state`,
  `group:upsert`/`remove`, `unit:upsert`/`remove`, `command:ack`). Global state is a
  `Mutex<Option<ServerHandle>>` — commands other than `start` fail with "not started"
  until it's set.
- `src/convert.rs` — Arma value <-> protocol/domain type conversions (side strings, unit
  category, damage->health, wind vector -> speed/direction, Arma `date` -> unix time).
- `src/sink.rs` — `ArmaCommandSink`: turns a dispatched `minerva_server::Command` into an
  `ExtensionCallback` (`context.callback_data("minerva", "command", [id, type, groupId, ...])`),
  picked up by `fnc_onCallback`/`fnc_executeCommand` on the SQF side.
- `addons/main/` — the SQF side (CBA-macro layout, mirrors `arma3-dynops`'s `addons/main`).

## The `minerva-server` dependency

Consumed as a `git` dependency (`Cargo.toml`), currently pinned to its `feat/scaffold`
branch because that PR hasn't merged yet — switch to a tag/rev once it has.

For local development against a working copy, create a gitignored `.cargo/config.toml`:

```toml
[patch."https://github.com/voron-simulations/minerva-server"]
minerva-server = { path = "../minerva-server" }
```

(assumes `minerva-server` is checked out as a sibling directory of this repo).

## Testing

- `cargo test` — unit tests (`src/convert.rs`) plus `tests/extension.rs`, which drives the
  extension through `arma_rs`'s `testing::Extension` (i.e. exactly the string-marshalled
  calls SQF would make) and, for the gRPC-facing tests, a real tonic client against the
  server `start` actually binds.
- `cargo bench` — `benches/extension.rs`, the full `unit:upsert` call path (marshalling +
  `StateCache` write), since that runs once per tracked unit per tick in `fnc_pushState`.
- `cargo llvm-cov` — coverage (CI posts a summary to the job summary).

## Gotchas

- `arma_rs::testing::Extension::call` args are the same strings a real `callExtension`
  call would send: string arguments must be quoted (`"\"g1\""`, not `"g1"`), everything
  else (numbers, arrays, bools) doesn't need to be.
- `arma_rs::Value` doesn't implement `IntoArma` directly — wrap it with
  `Value::direct(value)` when passing a pre-built `Value` to `callback_data`/as a command
  return value.
- `arma_rs::Extension`/`Context` hold an `Rc` internally, so they're `!Send`. Don't
  `tokio::spawn`/`spawn_blocking` anything that captures them — call their (sync, and in
  `callback_handler`'s case, blocking) methods directly in the current task instead; see
  `tests/extension.rs`'s `send_command_acked_via_callback_and_ack` for the pattern (spawn
  only the tonic client call, block the current task on `callback_handler`).

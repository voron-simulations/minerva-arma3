# AGENTS.md

Thin arma-rs + SQF wrapper around [`minerva-server`](https://github.com/voron-simulations/minerva-server):
pushes Arma state into its `StateCache` and executes commands it dispatches back.

## Layout

- `src/lib.rs` — the `#[arma] fn init()` entry point and the command handlers arma-rs
  routes SQF `callExtension` calls to (`start`, `stop`, `reset`, `sim:info`, `sim:state`,
  `group:upsert`/`remove`, `unit:remove`, `command:ack`). Global state is a
  `Mutex<Option<ServerHandle>>` — commands other than `start` fail with "not started"
  until it's set. There is no `unit:upsert`: a unit's state only ever arrives embedded in
  its group's `group:upsert` (see below) -- `unit:remove` stays standalone, for the
  immediate removal `XEH_postInit.sqf`'s `EntityKilled`/`EntityDeleted` handlers send.
- `src/convert.rs` — Arma value <-> protocol/domain type conversions (side strings, unit
  category, damage->health, wind vector -> speed/direction, Arma `date` -> unix time,
  `unit_from_parts` building the `Unit` embedded in a `group:upsert`).
- `src/sink.rs` — `ArmaCommandSink`: turns a dispatched `minerva_server::Command` into an
  `ExtensionCallback` (`context.callback_data("minerva", "command", [id, type, groupId, ...])`),
  picked up by `fnc_onCallback`/`fnc_executeCommand` on the SQF side.
- `addons/main/` — the SQF side (CBA-macro layout, mirrors `arma3-dynops`'s `addons/main`).
  `fnc_pushGroup.sqf` builds and sends one `group:upsert` call per group (readiness,
  waypoints, and its full tracked-unit list together). `fnc_schedulePush.sqf` is the
  per-frame handler that calls it: rather than pushing every group in the same frame, it
  spreads one push cycle (every eligible group, once) evenly across `TICK` seconds, and
  drains `fnc_requestGroupPush.sqf`'s dirty-group queue first each frame, for an
  event-driven push that can't wait for its regular turn (currently: a kill, so readiness
  and membership reflect it within a frame instead of up to `TICK` seconds later).

## `group:upsert`'s shape

```
["group:upsert", [id, side, [fuel, ammo, health], hasTask, waypoints, units]] call FUNC(call);
```

`units` is `[[id, kind, type, posASL, dir, velocity, damage], ...]` -- no `group_id` per
unit: the server fills it in from the containing group
(`StateCache::upsert_group_with_units`), so an embedded unit can't disagree with which
group it's actually in. It's the group's *complete* current membership: any unit
previously reported under this group but missing from this call's list is dropped, and
`fnc_pushGroup.sqf` builds it from `units _group` filtered to `alive` (a unit that died
this tick is dropped here too, not upserted one last time -- `unit:remove` already
reported it immediately). A crewed vehicle's whole crew maps to one `VEHICLE`/`HELICOPTER`/
`PLANE` entry (see `fnc_vehicleOwner.sqf`), not one entry per crewman. `XEH_postInit.sqf`'s
`EntityKilled` handler re-derives a killed crew member's vehicle's owner and dirties that
group too, since a kill can hand the vehicle to another group's surviving crew.

## The `minerva-server` dependency

Consumed as a `git` dependency (`Cargo.toml`) pinned to a `main` commit `rev` (it has no
tags yet); bump the `rev` to pick up server changes.

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
- `cargo bench` — `benches/extension.rs`, the full `group:upsert` call path (marshalling +
  `StateCache` write) for a 12-man-squad-plus-vehicle group, since that's the largest
  single call `fnc_pushGroup.sqf` makes, once per group per its turn in the push cycle.
- `cargo llvm-cov` — coverage (CI posts a summary to the job summary).
- `hemtt check` — lints the SQF (`.hemtt/lints.toml`) and rapifies the addon config; run
  before opening a PR, since CI's `hemtt release` build would otherwise be the first
  place a lint or config error surfaces.

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

## Docs

- `docs/adapter.md` — this adapter's specific behavior within the protocol contract
  (`minerva-protocol`'s `docs/contract.md`): what it reports for fields the contract
  leaves up to the server, and which commands it doesn't implement yet.
- `docs/in-game-testing.md` — runbook for an actual in-game smoke test against a real
  Arma 3 client.

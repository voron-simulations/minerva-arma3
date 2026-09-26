# minerva-arma3 adapter behavior

[`minerva-protocol`'s `docs/contract.md`](https://github.com/voron-simulations/minerva-protocol/blob/main/docs/contract.md)
defines what any client may rely on. This document covers only what's specific to this
adapter -- an Arma 3 client should not assume another `minerva-server` engine plugin
behaves the same way on any of these points.

## Coordinates and time

- `SimulationInfo.world_size` reports Arma's `worldSize` for both `x_meters` and
  `y_meters` -- the current adapter doesn't distinguish a non-square map.
- Timestamps are derived from Arma's `date` array, which has no seconds component: every
  `simulation_start_*_time`/`SimulationStateUpdate.simulation_time` is truncated to the
  minute.

## Readiness and loadout

- `ammo_state` is always `1`: there's no generic "ammo fraction" command for a mixed
  group, so no ammo model exists yet.
- `fuel_state` averages `fuel` over tracked vehicles (`1` if there are none) and
  `health_state` averages `1 - damage` over every tracked unit; a wrecked vehicle with an
  unhurt crew pulls the group's health down, not just its fuel.
- `UnitState.loadout` is never populated.

## Commands

- `DefendZone` and `Support` always return `COMMAND_RESULT_FAILURE` ("not implemented").
- `Move`/`SearchAndDestroy`/`Patrol` place a normal `"MOVE"`/`"SAD"` waypoint. A
  `CommandTarget` with no `z` is placed at the surface (`addWaypoint`'s ordinary AGL
  interpretation, `0` meaning the ground at that x/y); a present `z` is honored as an
  exact ASL altitude via `addWaypoint`'s `placementRadius = -1` (see
  `fnc_executeCommand.sqf`).

## Membership and units

- `ListLocations` is always empty: nothing in this adapter ever calls `set_locations`.
- Factions (`SimulationInfo.factions`) are empty unless mission code sets `GVAR(factions)`
  before `postInit` runs (there's no generic way to enumerate "the factions in play" from
  SQF alone).
- A crewed vehicle is reported as a single unit belonging to one deterministic owning
  group (the group of the first real crew seat -- driver, then commander/gunner/turrets),
  not one unit per crew member and not shared between the crew's group and any group
  riding as passengers.

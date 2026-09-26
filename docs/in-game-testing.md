# In-game testing

Manual smoke test of Arma 3 (Windows) <-> `minerva-server` <-> SunTzu (WSL; with mirrored
networking, `127.0.0.1:50051` is reachable from both sides). There is no automation for
this -- `cargo test`/`hemtt check` cover the extension and its SQF in isolation, not a
real client end to end.

## Setup

1. **Get the mod.** Download the `addon` artifact from a CI run of this branch (or
   `main`, once merged) on GitHub Actions -- built by `reusable-build.yml`'s "Build addon"
   job via `hemtt release`. Do not use the nightly release; it predates the
   `minerva-server` rebuild (#130/#131) and won't have `group:upsert`'s units arg. Unzip
   it; it contains an `@minerva` folder.
2. **Install it.** Copy `@minerva` into your Arma 3 install directory (next to `@cba_a3`,
   which you also need -- <https://steamcommunity.com/sharedfiles/filedetails/?id=450814997>).
   The extension is unsigned, so **BattlEye must be disabled** for this profile/launch
   (`-noBattlEye`, or disable it in the launcher) -- otherwise the game silently refuses to
   load `minerva_x64.dll`.
3. **Copy the test mission.** Copy `.hemtt/missions/minerva_test.VR/` into your Arma 3
   profile's `missions/` folder (typically
   `Documents/Arma 3/missions/minerva_test.VR/`). It spawns one infantry squad per side
   plus a crewed tank, and enough extra BLUFOR squads to bring the mission to ~50 groups
   (see its `init.sqf`).
4. **Launch Arma 3** with `@minerva` and `@cba_a3` enabled and BattlEye off, then **Editor
   -> VR -> minerva_test -> Preview**.
5. **Run SunTzu** in WSL: `dotnet run --project src/SunTzu.Web` (real client -- omit
   `--mock`). Default endpoint `http://127.0.0.1:50051` and side `BLUFOR` should just work;
   override via `appsettings.json` or the command line if your setup differs.

## Checklist

- Arma's `.rpt` log (`%localappdata%\Arma 3\<profile>.rpt`, or the profile's own folder)
  has no `Failed to invoke function` for anything under `minerva`.
- SunTzu's map shows groups and units scaled to the VR world's actual size, north up.
- **Staggering**: with ~50 groups in the mission, pushes should be spread across roughly a
  second, not all landing in the same instant. A temporary `diag_log` in
  `fnc_schedulePush.sqf` (or watching `GVAR(cycleCursor)` in the debug console) should
  show it climbing steadily across several frames per cycle, not jumping straight to the
  group count.
- Kill a unit (debug console: `player setDamage 1` on a non-player, or the "Kill" module):
  it disappears from its group within a frame (not up to a second later), the group's
  readiness updates, and it does not reappear on a later push.
- Delete a unit (`deleteVehicle`), and have a unit board/leave the tank: membership
  updates on both sides with no ghosts (a unit shown as both a lone soldier and part of
  the vehicle's crew, or lingering after removal).
- Click-to-move on SunTzu's map: a waypoint appears in Arma at the clicked spot (on the
  ground -- the map doesn't know terrain height), and SunTzu shows the command result as
  SUCCESS. A DefendZone command shows FAILURE with a reason (not implemented, see
  `docs/adapter.md`).
- Restart the mission preview: SunTzu's map clears and repopulates from the fresh
  simulation state, rather than showing stale groups from the previous run.
- **Large-group check**: the crewed tank's group and the 8-man squads should all push
  without a `callExtension` argument-size error in the `.rpt`; watch the frame time in the
  debug console (`diag_fps`/`diag_frameNo`) while previewing to sanity-check
  `fnc_pushGroup.sqf`'s per-frame cost isn't visible against everything else running.

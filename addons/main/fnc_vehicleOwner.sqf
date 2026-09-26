#include "script_component.hpp"
/*
    Returns the group that owns a vehicle's record: that of the first
    living unit in a real crew seat (driver, then commander/gunner/turrets
    in fullCrew's fixed order), or grpNull if only passengers (cargo/FFV)
    are aboard, or every real-seat occupant is dead. One deterministic
    owner per vehicle stops two groups sharing it -- as crew or as
    passengers -- from alternately upserting it under their own group ids.

    Shared by fnc_pushGroup.sqf (per-push ownership check) and
    XEH_postInit.sqf's EntityKilled handler (re-dirtying the vehicle's new
    owner immediately after a crew kill transfers it) so the two can't
    drift out of sync on what "owns" a vehicle.
*/
params ["_vehicle"];
private _seats = fullCrew [_vehicle, "", false];
// A dead occupant must not win the seat search: a live crewman from
// another group would then fail the caller's ownership check and the
// vehicle would go unreported by anyone until the corpse leaves the seat.
private _crewSeat = _seats findIf {(_x select 1) != "cargo" && {!(_x select 4)} && {alive (_x select 0)}};
if (_crewSeat < 0) exitWith {grpNull};
group (_seats select _crewSeat select 0)

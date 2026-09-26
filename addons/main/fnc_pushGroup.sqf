#include "script_component.hpp"
/*
    Pushes one group's (and its tracked units') current state into
    minerva-server via a single "group:upsert" call. Called once per group
    per push cycle by fnc_schedulePush, and immediately (out of its regular
    turn) by fnc_requestGroupPush.

    IDs are `[x] call BIS_fnc_netId`, stable across save/load and correct
    in both SP and MP (unlike the native `netId` command).
*/
params ["_group"];

private _fnc_avg = {
    params ["_values", ["_default", 1]];
    if (_values isEqualTo []) exitWith {_default};
    private _sum = 0;
    {_sum = _sum + _x} forEach _values;
    _sum / (count _values);
};

// Returns the group that owns a vehicle's record: that of the first unit
// in a real crew seat (driver, then commander/gunner/turrets in fullCrew's
// fixed order), or grpNull if only passengers (cargo/FFV) are aboard. One
// deterministic owner per vehicle stops two groups sharing it -- as crew or
// as passengers -- from alternately upserting it under their own group ids.
private _fnc_vehicleOwner = {
    params ["_vehicle"];
    private _seats = fullCrew [_vehicle, "", false];
    private _crewSeat = _seats findIf {(_x select 1) != "cargo" && {!(_x select 4)}};
    if (_crewSeat < 0) exitWith {grpNull};
    group (_seats select _crewSeat select 0)
};

private _groupId = [_group] call BIS_fnc_netId;

private _waypoints = [];
{
    // waypointType already returns a plain string; str() would add a
    // second layer of quoting that never matches convert::waypoint_type_from_str.
    // waypointPosition is always AGL regardless of how the waypoint was
    // placed; convert to ASL to match the protocol's positions (and
    // fnc_executeCommand.sqf's addWaypoint, which expects ASL back).
    _waypoints pushBack [waypointType _x, AGLToASL (waypointPosition _x)];
} forEach (waypoints _group);

// No generic "ammo fraction" command exists for a mixed group; report
// full until a real ammo model is built.
private _ammoState = 1;
// Completed waypoints stay in `waypoints _group`; once the last one is
// done currentWaypoint moves one past the end, so only that means idle.
private _hasTask = (currentWaypoint _group) < (count _waypoints);

// `units _group` gives the group's members, which for a crewed vehicle
// are the crew (persons), not the vehicle -- report the vehicle itself
// for members of its owning group, deduplicated (a tank's whole crew
// maps to one vehicle), and the person directly for everyone else.
private _trackedUnits = [];
{
    private _unit = _x;
    if (isNull objectParent _unit) then {
        _trackedUnits pushBackUnique _unit;
    } else {
        private _vehicle = objectParent _unit;
        private _ownsVehicle = ([_vehicle] call _fnc_vehicleOwner) isEqualTo _group;
        _trackedUnits pushBackUnique ([_unit, _vehicle] select _ownsVehicle);
    };
} forEach (units _group);
// A unit that died this tick is dropped here rather than upserted one last
// time: EntityKilled already sent its immediate unit:remove, and a dead
// unit lingers in `units _group` for a while after death, so leaving it in
// would otherwise re-add it as a live-looking member on this group's very
// next push (member replacement in upsert_group_with_units only drops what
// this call doesn't list).
_trackedUnits = _trackedUnits select {alive _x};

// Fuel and health are over the same entities as the unit records below,
// so a wrecked tank with an unhurt crew doesn't report a healthy group,
// and a ride in another group's vehicle doesn't count as this one's fuel.
private _fuelState = [_trackedUnits select {!(_x isKindOf "Man")} apply {fuel _x}] call _fnc_avg;
private _healthState = 1 - ([_trackedUnits apply {damage _x}] call _fnc_avg);

private _units = _trackedUnits apply {
    private _unit = _x;
    // `Man` inherits from `AllVehicles` in Arma's class hierarchy, so it
    // must be excluded explicitly or every soldier is misclassified as
    // a generic VEHICLE below.
    private _kind = "INFANTRY";
    if (_unit isKindOf "Helicopter") then {
        _kind = "HELICOPTER";
    } else {
        if (_unit isKindOf "Plane") then {
            _kind = "PLANE";
        } else {
            if (_unit isKindOf "AllVehicles" && {!(_unit isKindOf "Man")}) then {_kind = "VEHICLE"};
        };
    };
    [
        [_unit] call BIS_fnc_netId,
        _kind,
        typeOf _unit,
        getPosASL _unit,
        getDir _unit,
        velocity _unit,
        damage _unit
    ]
};

[
    "group:upsert",
    [_groupId, str (side _group), [_fuelState, _ammoState, _healthState], _hasTask, _waypoints, _units]
] call FUNC(call);

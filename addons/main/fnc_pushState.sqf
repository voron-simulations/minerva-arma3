#include "script_component.hpp"
/*
    Pushes the current simulation/group/unit state into minerva-server.
    Runs every TICK from the server's per-frame handler (see XEH_postInit).

    IDs are `[x] call BIS_fnc_netId`, stable across save/load and correct
    in both SP and MP (unlike the native `netId` command).
*/

private _fnc_avg = {
    params ["_values", ["_default", 1]];
    if (_values isEqualTo []) exitWith {_default};
    private _sum = 0;
    {_sum = _sum + _x} forEach _values;
    _sum / (count _values);
};

["sim:state", [date, accTime, overcast, wind]] call FUNC(call);

private _emittedUnitIds = [];

{
    private _group = _x;
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

    private _fuelLevels = (units _group) select {!isNull objectParent _x} apply {fuel (vehicle _x)};
    private _fuelState = [_fuelLevels] call _fnc_avg;
    // No generic "ammo fraction" command exists for a mixed group; report
    // full until a real ammo model is built.
    private _ammoState = 1;
    private _healthState = 1 - ([(units _group) apply {damage _x}] call _fnc_avg);
    // The group's last waypoint is never removed by the engine, so a
    // nonempty array doesn't mean there's still work left -- only an
    // unreached one does.
    private _hasTask = (currentWaypoint _group) < ((count _waypoints) - 1);

    [
        "group:upsert",
        [_groupId, str (side _group), [_fuelState, _ammoState, _healthState], _hasTask, _waypoints]
    ] call FUNC(call);

    // `units _group` gives the group's members, which for a crewed vehicle
    // are the crew (persons), not the vehicle -- report the vehicle itself
    // for mounted members, deduplicated (a tank's whole crew maps to one
    // vehicle), and the person directly for dismounted ones.
    private _trackedUnits = [];
    {
        private _unit = _x;
        if (isNull objectParent _unit) then {
            _trackedUnits pushBackUnique _unit;
        } else {
            // Only the vehicle's actual crew (driver/gunner/commander)
            // upserts it as their group's unit; a member merely riding as
            // cargo in another group's vehicle is tracked as themselves,
            // so two groups sharing a ride don't fight over who owns the
            // vehicle record in the cache.
            private _assignment = assignedVehicleRole _unit;
            private _isCrew = (_assignment isNotEqualTo []) && {(_assignment select 0) != "Cargo"};
            _trackedUnits pushBackUnique (if (_isCrew) then {vehicle _unit} else {_unit});
        };
    } forEach (units _group);

    {
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

        private _unitId = [_unit] call BIS_fnc_netId;
        _emittedUnitIds pushBack _unitId;
        [
            "unit:upsert",
            [_unitId, _groupId, _kind, typeOf _unit, getPosASL _unit, getDir _unit, velocity _unit, damage _unit]
        ] call FUNC(call);
    } forEach _trackedUnits;
} forEach allGroups;

// A unit whose emitted id changed since last tick (e.g. it boarded or left
// a vehicle, switching between its own id and its transport's) would
// otherwise leave its old representation in the cache forever, since this
// function only ever upserts.
{
    if !(_x in _emittedUnitIds) then {
        ["unit:remove", [_x]] call FUNC(call);
    };
} forEach GVAR(trackedUnitIds);
GVAR(trackedUnitIds) = _emittedUnitIds;

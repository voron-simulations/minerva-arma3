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

{
    private _group = _x;
    private _groupId = [_group] call BIS_fnc_netId;

    private _waypoints = [];
    {
        // waypointType already returns a plain string; str() would add a
        // second layer of quoting that never matches convert::waypoint_type_from_str.
        _waypoints pushBack [waypointType _x, waypointPosition _x];
    } forEach (waypoints _group);

    private _fuelLevels = (units _group) select {!isNull objectParent _x} apply {fuel (vehicle _x)};
    private _fuelState = [_fuelLevels] call _fnc_avg;
    // No generic "ammo fraction" command exists for a mixed group; report
    // full until a real ammo model is built.
    private _ammoState = 1;
    private _healthState = 1 - ([(units _group) apply {damage _x}] call _fnc_avg);

    [
        "group:upsert",
        [_groupId, str (side _group), [_fuelState, _ammoState, _healthState], _waypoints isNotEqualTo [], _waypoints]
    ] call FUNC(call);

    // `units _group` gives the group's members, which for a crewed vehicle
    // are the crew (persons), not the vehicle -- report the vehicle itself
    // for mounted members, deduplicated (a tank's whole crew maps to one
    // vehicle), and the person directly for dismounted ones.
    private _trackedUnits = [];
    {
        _trackedUnits pushBackUnique (if (isNull objectParent _x) then {_x} else {vehicle _x});
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

        [
            "unit:upsert",
            [[_unit] call BIS_fnc_netId, _groupId, _kind, typeOf _unit, getPosASL _unit, getDir _unit, velocity _unit, damage _unit]
        ] call FUNC(call);
    } forEach _trackedUnits;
} forEach allGroups;

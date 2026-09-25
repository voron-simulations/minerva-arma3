#include "script_component.hpp"
/*
    Executes a command dispatched by minerva-server, decoded from a
    "minerva"/"command" ExtensionCallback payload: [id, type, groupId, ...],
    see ArmaCommandSink (minerva-arma3's src/sink.rs) for the exact shape
    per type. Always ends with a command:ack.
*/
params ["_id", "_type", "_groupId"];
private _extra = _this select [3];

private _fnc_ack = {
    params ["_id", "_ok", ["_reason", ""]];
    ["command:ack", [_id, _ok, _reason]] call FUNC(call);
};

private _group = _groupId call BIS_fnc_groupFromNetId;
if (isNull _group) exitWith {
    [_id, false, "unknown group"] call _fnc_ack;
};

private _fnc_clearWaypoints = {
    params ["_grp"];
    for "_i" from (count waypoints _grp) - 1 to 0 step -1 do {
        deleteWaypoint [_grp, _i];
    };
};

switch (_type) do {
    case "move": {
        _extra params ["_x", "_y", "_z"];
        _group call _fnc_clearWaypoints;
        // placementRadius -1 makes addWaypoint honor this position's z as
        // ASL (matching getPosASL, what the protocol's positions are in)
        // instead of AGL, its normal interpretation.
        (_group addWaypoint [[_x, _y, _z], -1]) setWaypointType "MOVE";
        [_id, true] call _fnc_ack;
    };
    case "search_and_destroy": {
        _extra params ["_x", "_y", "_z"];
        _group call _fnc_clearWaypoints;
        (_group addWaypoint [[_x, _y, _z], -1]) setWaypointType "SAD";
        [_id, true] call _fnc_ack;
    };
    case "patrol": {
        _extra params ["_loop"];
        private _coords = _extra select [1];
        private _count = (count _coords) / 3;
        _group call _fnc_clearWaypoints;
        private _lastPos = [];
        for "_i" from 0 to (_count - 1) do {
            _lastPos = _coords select [_i * 3, 3];
            (_group addWaypoint [_lastPos, -1]) setWaypointType "MOVE";
        };
        if (_loop) then {
            // CYCLE is a control waypoint that redirects back to the first
            // one; it's appended after every requested point rather than
            // replacing the last one, which would otherwise skip it.
            (_group addWaypoint [_lastPos, -1]) setWaypointType "CYCLE";
        };
        [_id, true] call _fnc_ack;
    };
    case "defend_zone": {
        [_id, false, "not implemented"] call _fnc_ack;
    };
    case "support": {
        [_id, false, "not implemented"] call _fnc_ack;
    };
    default {
        [_id, false, format ["unknown command type: %1", _type]] call _fnc_ack;
    };
};

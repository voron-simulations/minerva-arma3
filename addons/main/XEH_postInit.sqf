#include "script_component.hpp"

if (!isServer) exitWith {};

if (isNil QGVAR(listenAddress)) then {
    GVAR(listenAddress) = "127.0.0.1:50051";
};
// Mission code can set this (before postInit runs) to report real factions;
// there's no generic way to enumerate "the factions in play" from SQF alone.
if (isNil QGVAR(factions)) then {
    GVAR(factions) = [];
};
// fnc_schedulePush's push cycle: the (snapshotted) groups still due a push
// this cycle, and where in it the cursor is.
GVAR(cycle) = [];
GVAR(cycleCursor) = 0;
// Groups fnc_requestGroupPush has marked for an out-of-band push on the
// very next frame.
GVAR(dirtyGroups) = [];

["start", [GVAR(listenAddress)]] call FUNC(call);
["reset"] call FUNC(call);
["sim:info", [worldName, [worldSize, worldSize], GVAR(factions), date]] call FUNC(call);

addMissionEventHandler ["ExtensionCallback", FUNC(onCallback)];

addMissionEventHandler ["EntityKilled", {
    params ["_unit"];
    if (_unit isKindOf "Man" || {_unit isKindOf "AllVehicles"}) then {
        ["unit:remove", [[_unit] call BIS_fnc_netId]] call FUNC(call);
        // Readiness (health/fuel averages) and membership shouldn't wait
        // up to TICK seconds to reflect a kill.
        [group _unit] call FUNC(requestGroupPush);
        // A dead real-seat occupant can hand the vehicle's record to
        // another group's surviving crew (fnc_vehicleOwner, recomputed
        // here now that _unit is already dead); that group's own regular
        // turn is up to TICK away, so it needs the same out-of-band push
        // as the group that just lost the vehicle.
        private _vehicle = objectParent _unit;
        if (!isNull _vehicle) then {
            [[_vehicle] call FUNC(vehicleOwner)] call FUNC(requestGroupPush);
        };
    };
}];

addMissionEventHandler ["EntityDeleted", {
    params ["_entity"];
    if (_entity isKindOf "Man" || {_entity isKindOf "AllVehicles"}) then {
        ["unit:remove", [[_entity] call BIS_fnc_netId]] call FUNC(call);
    };
}];

addMissionEventHandler ["GroupDeleted", {
    params ["_group"];
    ["group:remove", [[_group] call BIS_fnc_netId]] call FUNC(call);
}];

addMissionEventHandler ["Ended", {["reset"] call FUNC(call);}];
addMissionEventHandler ["MPEnded", {["reset"] call FUNC(call);}];

[FUNC(schedulePush), 0, []] call CBA_fnc_addPerFrameHandler;

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
// fnc_pushState's previous-tick emitted unit ids, so it can detect ones
// that stopped being emitted (e.g. a unit that boarded or left a vehicle,
// switching between its own id and its transport's) and remove them.
GVAR(trackedUnitIds) = [];

["start", [GVAR(listenAddress)]] call FUNC(call);
["reset"] call FUNC(call);
["sim:info", [worldName, [worldSize, worldSize], GVAR(factions), date]] call FUNC(call);

addMissionEventHandler ["ExtensionCallback", FUNC(onCallback)];

addMissionEventHandler ["EntityKilled", {
    params ["_unit"];
    if (_unit isKindOf "Man" || {_unit isKindOf "AllVehicles"}) then {
        ["unit:remove", [[_unit] call BIS_fnc_netId]] call FUNC(call);
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

[FUNC(pushState), TICK, []] call CBA_fnc_addPerFrameHandler;

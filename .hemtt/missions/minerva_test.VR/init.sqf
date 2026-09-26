/*
    minerva_test.VR: a scratch mission for the in-game smoke test (see
    docs/in-game-testing.md). Spawns one infantry squad per side (so
    SunTzu's map shows all three factions) plus a crewed vehicle (exercises
    fnc_pushGroup's vehicle-ownership/deduplication path -- the crew reports
    as the vehicle, not as themselves), and enough extra BLUFOR squads to
    push the mission to a mission-scale group count, so fnc_schedulePush's
    stagger is actually exercised rather than trivially finishing a tiny
    cycle in a single frame.
*/
if (!isServer) exitWith {};

private _fnc_infantrySquad = {
    params ["_side", "_class", "_origin", "_count"];
    private _group = createGroup [_side, true];
    for "_i" from 0 to (_count - 1) do {
        private _pos = _origin vectorAdd [(_i % 4) * 3, floor (_i / 4) * 3, 0];
        _group createUnit [_class, _pos, [], 0, "NONE"];
    };
    _group
};

[west, "B_Soldier_F", [0, 0, 0], 8] call _fnc_infantrySquad;
[east, "O_Soldier_F", [200, 0, 0], 8] call _fnc_infantrySquad;
[independent, "I_Soldier_F", [0, 200, 0], 8] call _fnc_infantrySquad;

private _tank = "B_MBT_01_cannon_F" createVehicle [50, 50, 0];
_tank setDir 90;
createVehicleCrew _tank;

// ~50 groups total across the mission: enough to make each fnc_schedulePush
// cycle span multiple frames rather than complete in one, so staggering is
// actually observable (see docs/in-game-testing.md's checklist).
for "_g" from 0 to 45 do {
    private _origin = [400 + (_g % 10) * 30, (floor (_g / 10)) * 30, 0];
    [west, "B_Soldier_F", _origin, 4] call _fnc_infantrySquad;
};

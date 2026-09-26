#include "script_component.hpp"
/*
    Added with delay 0 by XEH_postInit, so CBA runs this every frame (0
    means "as often as possible", not "once"). Spreads each period's full
    round of group pushes evenly across ~TICK seconds instead of pushing
    every group in the same frame -- which would land every group's
    callExtension cost in a single frame's budget on a mission with many
    groups -- and pushes any group marked dirty (see fnc_requestGroupPush)
    on the very next frame, ahead of its regular turn in the cycle.

    diag_tickTime is real (wall-clock) time, unaffected by time
    acceleration: TICK means "this often in real seconds", not simulation
    time, so pushing on simulation time would make the stagger -- and the
    push rate itself -- speed up or slow down with time acceleration.
*/

{
    if !(isNull _x) then {
        [_x] call FUNC(pushGroup);
    };
} forEach GVAR(dirtyGroups);
GVAR(dirtyGroups) = [];

if (GVAR(cycleCursor) >= count GVAR(cycle)) then {
    if (isNil QGVAR(cycleStart) || {diag_tickTime - GVAR(cycleStart) >= TICK}) then {
        GVAR(cycle) = allGroups select {
            // sideLogic groups (modules, headless clients) have no
            // protocol side; publishing them only produces rejected group
            // upserts and orphaned units.
            (side _x) in [west, east, independent, civilian]
        };
        GVAR(cycleCursor) = 0;
        GVAR(cycleStart) = diag_tickTime;
        ["sim:state", [date, accTime, overcast, wind]] call FUNC(call);
    };
};

private _cycleCount = count GVAR(cycle);
if (GVAR(cycleCursor) < _cycleCount) then {
    private _elapsed = (diag_tickTime - GVAR(cycleStart)) min TICK;
    private _due = 0 max ceil (_cycleCount * _elapsed / TICK);
    while {GVAR(cycleCursor) < _due} do {
        private _group = GVAR(cycle) select GVAR(cycleCursor);
        if !(isNull _group) then {
            [_group] call FUNC(pushGroup);
        };
        GVAR(cycleCursor) = GVAR(cycleCursor) + 1;
    };
};

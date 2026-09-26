#include "script_component.hpp"
/*
    Marks a group for an out-of-band push on the very next frame, instead of
    waiting for its regular turn in fnc_schedulePush's staggered cycle (up
    to TICK seconds away). Call this from an event handler whose effect on
    readiness or membership shouldn't have to wait a full tick to show up
    (e.g. EntityKilled, alongside its immediate unit:remove).
*/
params ["_group"];
if (isNull _group) exitWith {};
GVAR(dirtyGroups) pushBackUnique _group;

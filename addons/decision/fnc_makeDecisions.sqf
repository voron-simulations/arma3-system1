#include "script_component.hpp"

/*
 * Runs the AI decision loop for a group until the group is dead.
 * Usage: [_group] spawn System1_fnc_makeDecisions
 * Arguments: 0: group <GROUP>
 */
params [["_group", grpNull, [grpNull]]];

if (isNull _group) exitWith {
    ["system1: makeDecisions called with a null group"] call BIS_fnc_error;
    ERROR("makeDecisions called with a null group");
};
if (!local _group) exitWith {
    ["system1: makeDecisions: group %1 is not local, run it where the group is local", _group] call BIS_fnc_error;
    ERROR_1("group %1 is not local",_group);
};
if (!isNil { _group getVariable QGVAR(id) }) exitWith {
    WARNING_1("group %1 already has a decision loop",_group);
};

private _id = GVAR(nextId);
GVAR(nextId) = _id + 1;
GVAR(groups) set [_id, _group];
_group setVariable [QGVAR(id), _id];
_group setVariable [QGVAR(initialSize), count (units _group select { alive _x })];
_group setVariable [QGVAR(initialRounds), _group call FUNCMAIN(countRounds)];

while { units _group findIf { alive _x } >= 0 } do {
    sleep GVARMAIN(interval);

    // One request at a time per group; a slow model must not queue up stale decisions.
    if (_group getVariable [QGVAR(inFlight), false]) then { continue };
    // Nothing to decide about without known enemies.
    if ((leader _group) targets [true] isEqualTo []) then { continue };

    _group setVariable [QGVAR(inFlight), true];
    private _snapshot = _group call FUNCMAIN(collectContext);
    private _result = ["decide", [GVARMAIN(endpoint), _id, _snapshot, GVARMAIN(maxContacts)]] call System1_fnc_call;
    // fnc_call returns the extension result, "OK" when the request was accepted.
    if (_result isNotEqualTo "OK") then { _group setVariable [QGVAR(inFlight), false] };
};

GVAR(groups) deleteAt _id;

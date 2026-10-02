#include "script_component.hpp"

/*
 * Reports a failed decision request. The loop keeps running and retries on its next tick.
 * Arguments: 0: group id <NUMBER>, 1: HTTP status, 0 if there was no HTTP response <NUMBER>, 2: detail <STRING>
 */
params ["_id", "_status", "_detail"];

private _group = GVAR(groups) getOrDefault [_id, grpNull];
if (!isNull _group) then { _group setVariable [QGVAR(inFlight), false] };

["system1: decision for group %1 failed: HTTP %2: %3", _id, _status, _detail] call BIS_fnc_error;
ERROR_3("decision for group %1 failed: HTTP %2: %3",_id,_status,_detail);

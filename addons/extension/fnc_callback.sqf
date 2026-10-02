#include "script_component.hpp"

params ["_name", "_component", "_data"];

if ((toLower _name) != "system1") exitWith { };

TRACE_2("Callback %1 called with args %2",_component,_data);

// Whitelist rather than dispatching by name: callback names come from outside SQF.
private _args = parseSimpleArray _data;
switch (_component) do {
    case "decision": { _args call System1_fnc_applyDecision; };
    case "error": { _args call System1_fnc_onDecisionError; };
    default { ERROR_1("Unknown callback '%1'",_component); };
};

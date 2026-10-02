#include "script_component.hpp"

// Server-side settings (isGlobal = 1): the decision loop only runs where the group is local.
[
    QGVARMAIN(endpoint), "EDITBOX",
    ["Endpoint", "Base URL of the decision model service"],
    "System1", "http://localhost:8000", 1
] call CBA_fnc_addSetting;

[
    QGVARMAIN(interval), "SLIDER",
    ["Decision interval (s)", "Seconds between decision requests per group"],
    "System1", [5, 300, 30, 0], 1
] call CBA_fnc_addSetting;

[
    QGVARMAIN(maxContacts), "SLIDER",
    ["Max contacts", "Contacts listed in the model context; the model rejects oversized contexts"],
    "System1", [1, 50, 8, 0], 1
] call CBA_fnc_addSetting;

INFO("PreInit finished");

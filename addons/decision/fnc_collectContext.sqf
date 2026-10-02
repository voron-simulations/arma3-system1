#include "script_component.hpp"

/*
 * Builds the positional array consumed by the extension's GroupSnapshot:
 * [[wpType, wpDist, wpDir], initialSize, [[damage, incapacitated], ...], ammoFraction, [[category, dist, dir], ...]]
 * Arguments: 0: group <GROUP>
 * Return: <ARRAY>
 */
params [["_group", grpNull, [grpNull]]];

private _leader = leader _group;

private _task = ["", 0, 0];
private _waypoints = waypoints _group;
private _wpIndex = currentWaypoint _group;
if (_wpIndex < count _waypoints) then {
    private _wp = _waypoints select _wpIndex;
    private _wpPos = waypointPosition _wp;
    _task = [waypointType _wp, round (_leader distance2D _wpPos), round (_leader getDir _wpPos)];
};

private _alive = units _group select { alive _x };
private _units = _alive apply { [damage _x, lifeState _x == "INCAPACITATED"] };

private _baseline = _group getVariable [QGVAR(initialRounds), 0];
private _ammo = if (_baseline > 0) then { (_group call FUNCMAIN(countRounds)) / _baseline } else { 0 };

private _contacts = (_leader targets [true]) apply {
    // Tank is a LandVehicle too, so the more specific kinds go first.
    private _category = switch (true) do {
        case (_x isKindOf "Man"): { "inf" };
        case (_x isKindOf "Tank"): { "armor" };
        case (_x isKindOf "Air"): { "air" };
        case (_x isKindOf "StaticWeapon"): { "static" };
        case (_x isKindOf "LandVehicle"): { "veh" };
        default { "other" };
    };
    [_category, round (_leader distance2D _x), round (_leader getDir _x)]
};

[_task, _group getVariable [QGVAR(initialSize), count _alive], _units, _ammo, _contacts]

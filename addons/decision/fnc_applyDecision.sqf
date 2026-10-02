#include "script_component.hpp"

/*
 * Applies a decision from the model by inserting a temporary waypoint.
 * Arguments: 0: group id <NUMBER>, 1: choice <STRING>, 2: probabilities [continue, retreat, flank] <ARRAY>, 3: confidence <NUMBER>
 */
params ["_id", "_choice", "_probabilities", "_confidence"];

private _group = GVAR(groups) getOrDefault [_id, grpNull];
if (isNull _group) exitWith {
    // The group died while the request was in flight.
    INFO_1("decision for unknown group id %1 dropped",_id);
};
_group setVariable [QGVAR(inFlight), false];

INFO_4("group %1 decision %2, probabilities %3, confidence %4",_group,_choice,_probabilities,_confidence);

// The previous maneuver is stale whatever the new choice is; reversed because deleting shifts indices.
private _stale = waypoints _group;
reverse _stale;
{
    if (waypointName _x == MANEUVER_WP_NAME) then { deleteWaypoint _x };
} forEach _stale;

if (_choice == "continue") exitWith {};

private _leader = leader _group;
private _enemies = _leader targets [true];
if (_enemies isEqualTo []) exitWith {
    INFO_1("group %1: no known enemies left, maneuver skipped",_group);
};

private _sum = [0, 0];
{
    private _p = getPosASL _x;
    _sum = [(_sum select 0) + (_p select 0), (_sum select 1) + (_p select 1)];
} forEach _enemies;
private _centroid = [(_sum select 0) / count _enemies, (_sum select 1) / count _enemies];
private _enemyBearing = _leader getDir _centroid;

private _target = [];
private _behaviour = "";
private _combatMode = "";
private _speed = "";
switch (_choice) do {
    case "retreat": {
        _target = _leader getPos [RETREAT_DISTANCE, _enemyBearing + 180];
        _behaviour = "AWARE";
        _speed = "FULL";
    };
    case "flank": {
        // Go round the side where fewer enemies are known.
        private _onRight = { ((_leader getDir _x) - _enemyBearing + 360) % 360 < 180 } count _enemies;
        private _sideBearing = _enemyBearing + ([-90, 90] select (_onRight <= count _enemies - _onRight));
        _target = (_leader getPos [FLANK_SIDE_DISTANCE, _sideBearing]) getPos [FLANK_FORWARD_DISTANCE, _enemyBearing];
        _behaviour = "COMBAT";
        _speed = "NORMAL";
    };
    default { ERROR_1("unknown decision '%1'",_choice); };
};

if (_target isEqualTo []) exitWith {};

// Insert before the group's own waypoints so that they resume once the maneuver is done.
private _index = currentWaypoint _group;
private _wp = _group addWaypoint [_target, 0, _index, MANEUVER_WP_NAME];
_wp setWaypointType "MOVE";
_wp setWaypointBehaviour _behaviour;
_wp setWaypointSpeed _speed;
_group setCurrentWaypoint _wp;

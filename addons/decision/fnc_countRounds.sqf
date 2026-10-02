#include "script_component.hpp"

/*
 * Total rounds across all magazines of the alive units of a group.
 * Arguments: 0: group <GROUP>
 * Return: <NUMBER>
 */
params [["_group", grpNull, [grpNull]]];

private _total = 0;
{
    { _total = _total + (_x select 1); } forEach magazinesAmmo _x;
} forEach (units _group select { alive _x });
_total

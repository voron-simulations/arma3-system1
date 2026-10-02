#include "script_component.hpp"

// id -> group, so that extension callbacks (which only carry the id) can find their group.
GVAR(groups) = createHashMap;
GVAR(nextId) = 0;

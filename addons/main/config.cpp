#include "script_component.hpp"

class CfgPatches {
    class ADDON {
        name = COMPONENT_NAME;
        units[] = {};
        weapons[] = {};
        requiredVersion = REQUIRED_VERSION;
        requiredAddons[] = {"cba_main"};
        VERSION_CONFIG;
    };
};

class CfgMods {
    class System1 {
        dir = "@system1";
        name = "System1";
        hidePicture = "true";
        hideName = "true";
        actionName = "Website";
        action = "https://github.com/voron-simulations/arma3-system1";
        description = "Issue Tracker: https://github.com/voron-simulations/arma3-system1/issues";
    };
};

#include "CfgEventHandlers.hpp"

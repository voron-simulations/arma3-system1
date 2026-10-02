#define MAINPREFIX dw
#define PREFIX system1

#include "script_version.hpp"

#define VERSION         MAJOR.MINOR.PATCH
#define VERSION_STR     MAJOR.MINOR.PATCH
#define VERSION_AR      MAJOR,MINOR,PATCH
#define VERSION_PLUGIN  MAJOR.MINOR.PATCH

#define REQUIRED_VERSION 2.06

#ifdef COMPONENT_BEAUTIFIED
    #define COMPONENT_NAME QUOTE(System1 COMPONENT_BEAUTIFIED)
#else
    #define COMPONENT_NAME QUOTE(System1 COMPONENT)
#endif

#include "script_macros_common.hpp"

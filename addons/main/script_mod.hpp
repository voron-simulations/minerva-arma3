#define MAINPREFIX dw
#define PREFIX minerva

#include "script_version.hpp"

// A bare MAJOR.MINOR.PATCH here (as arma3-dynops' script_mod.hpp defines it)
// fails HEMTT's invalid_value lint: `version = MAJOR.MINOR.PATCH;` isn't a
// valid Arma config number (two dots). VERSION_CONFIG only needs a plain
// number for `version`; the dotted form belongs to VERSION_STR/VERSION_AR,
// which VERSION_CONFIG already wraps as a string/array.
#define VERSION         MAJOR
#define VERSION_STR     MAJOR.MINOR.PATCH
#define VERSION_AR      MAJOR,MINOR,PATCH
#define VERSION_PLUGIN  MAJOR.MINOR.PATCH

// 2.18 for the EntityKilled/EntityDeleted mission event handlers (HEMTT's
// L-S02IV lint catches EHs newer than this).
#define REQUIRED_VERSION 2.18

#ifdef COMPONENT_BEAUTIFIED
    #define COMPONENT_NAME QUOTE(Minerva COMPONENT_BEAUTIFIED)
#else
    #define COMPONENT_NAME QUOTE(Minerva COMPONENT)
#endif

#include <\x\cba\addons\main\script_macros_common.hpp>

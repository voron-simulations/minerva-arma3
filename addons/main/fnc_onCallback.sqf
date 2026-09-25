#include "script_component.hpp"
/*
    Handles the "ExtensionCallback" mission event handler
    (https://community.bistudio.com/wiki/Arma_3:_Mission_Event_Handlers#ExtensionCallback)
    filtered to the "minerva" extension.

    Params:
        _name    - STRING, extension name, only "minerva" is handled
        _function - STRING, callback kind; only "command" is handled
        _data    - STRING, SQF-array literal (see fnc_executeCommand)
*/
params ["_name", "_function", "_data"];
if (_name != "minerva") exitWith {};

switch (_function) do {
    case "command": {
        (parseSimpleArray _data) call FUNC(executeCommand);
    };
};

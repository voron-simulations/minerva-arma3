private _function = param [0, "echo", [""]];
private _args = param [1, [], [[]]];

// Call the extension and process the error codes
private _resultArray = "minerva" callExtension [_function, _args];

_resultArray params ["_result", "_retcode", "_errcode"];
if (_errcode != 0 && _errcode != 301) then {
	switch (_errcode) do {
		case 101: { _errcode = "SYNTAX_ERROR_WRONG_PARAMS_SIZE"; };
		case 102: { _errcode = "SYNTAX_ERROR_WRONG_PARAMS_TYPE"; };
		case 201: { _errcode = "PARAMS_ERROR_TOO_MANY_ARGS"; };
	};

	["Failed to invoke function '%1': error %2", _function, _errcode] call BIS_fnc_error;
};
if (_retcode != 0) then {
	["Function '%1' returned code %2: %3", _function, _retcode, _result] call BIS_fnc_error;
};

// Return the unwrapped call result without the error codes
_result;

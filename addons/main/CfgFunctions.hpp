// The outer tag is ADDON (minerva_main), not the bare "minerva" prefix:
// Arma's CfgFunctions registers <tag>_fnc_<name>, ignoring the component
// nesting, so the tag must equal what FUNC() computes (PREFIX_COMPONENT)
// for `call FUNC(x)` to resolve to the function actually registered here.
class CfgFunctions {
    class ADDON {
        class COMPONENT {
            PATHTO_FNC(call);
            PATHTO_FNC(pushState);
            PATHTO_FNC(onCallback);
            PATHTO_FNC(executeCommand);
        };
    };
};

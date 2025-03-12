from os.path import join
from os import walk
import pathlib

addons_dir = pathlib.Path(__file__).parent.parent.joinpath('addons')
for dir in addons_dir.iterdir():
    cfgfunctions_path = dir.joinpath('CfgFunctions.hpp')
    functions = []

    contents = "class CfgFunctions {\n"
    contents += "    class Minerva {\n"
    contents += "        class core {\n"

    function_files = (f for f in dir.iterdir() 
                        if f.is_file() and f.suffix == '.sqf' and f.name.startswith('fnc_'))
    for file in function_files:
        function_name = file.name.removeprefix('fnc_').removesuffix('.sqf')
        contents += f"            class {function_name} {{}};\n"

    contents += "        };\n"
    contents += "    };\n"
    contents += "};\n"

    cfgfunctions_path.write_text(contents, encoding='utf8')

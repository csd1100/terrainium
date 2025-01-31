# This file is auto-generated by terrainium
# DO NOT EDIT MANUALLY USE `terrainium edit` COMMAND TO EDIT TOML

if [ -z "$TERRAINIUM_EXECUTABLE" ]; then
    TERRAINIUM_EXECUTABLE=terrainium
fi

function {
    # USER DEFINED ALIASES: START
    alias greet="echo hello"
    alias tenter="terrainium enter --biome example_biome"
    alias texit="terrainium exit"
    # USER DEFINED ALIASES: END
    # USER DEFINED ENVS: START
    export BIOME_POINTER="biome_real"
    export BIOME_REAL="biome_real"
    export EDITOR="nvim"
    export NESTED_POINTER="biome_value"
    export NULL_POINTER="$NULL"
    export PAGER="less"
    export POINTER="biome_value"
    export REAL="biome_value"
    # USER DEFINED ENVS: END
}

function __terrainium_shell_constructor() {
    if [ "$TERRAIN_ENABLED" = "true" ]; then
        /bin/echo entering terrain
        /bin/echo entering biome example_biome
    fi
}

function __terrainium_shell_destructor() {
    if [ "$TERRAIN_ENABLED" = "true" ]; then
        /bin/echo exiting terrain
        /bin/echo exiting biome example_biome
    fi
}

function __terrainium_enter() {
    __terrainium_shell_constructor
}

function __terrain_prompt() {
    if [ "$TERRAIN_ENABLED" = "true" ]; then
        echo "terrainium(example_biome)"
    fi
}

function __terrainium_exit() {
    if [ "$TERRAIN_ENABLED" = "true" ]; then
        builtin exit
    fi
}

function __terrainium_preexec_functions() {
    tenter="(\$TERRAINIUM_EXECUTABLE enter*|$TERRAINIUM_EXECUTABLE enter*|*terrainium enter*)"
    texit="(\$TERRAINIUM_EXECUTABLE exit*|$TERRAINIUM_EXECUTABLE exit*|*terrainium exit*)"
    tconstruct="(\$TERRAINIUM_EXECUTABLE construct*|$TERRAINIUM_EXECUTABLE construct*|*terrainium construct*)"
    tdestruct="(\$TERRAINIUM_EXECUTABLE destruct*|$TERRAINIUM_EXECUTABLE destruct*|*terrainium destruct*)"

    if [ $TERRAIN_ENABLED = "true" ]; then
        case "$3" in
            $~texit)
                __terrainium_exit
                ;;
            $~tconstruct)
                __terrainium_shell_constructor
                ;;
            $~tdestruct)
                __terrainium_shell_destructor
                ;;
        esac
    fi
}

function __terrainium_zshexit_functions() {
    __terrainium_shell_destructor
    "$TERRAINIUM_EXECUTABLE" exit
}

preexec_functions=(__terrainium_preexec_functions $preexec_functions)
zshexit_functions=(__terrainium_zshexit_functions $zshexit_functions)

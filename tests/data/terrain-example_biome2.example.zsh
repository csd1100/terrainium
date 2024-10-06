# This file is auto-generated by terrainium
# DO NOT EDIT MANUALLY USE `terrainium edit` COMMAND TO EDIT TOML

if [ -z "$TERRAINIUM_EXECUTABLE" ]; then
    TERRAINIUM_EXECUTABLE=terrainium
fi

function {
    # USER DEFINED ALIASES: START
    alias new="new_alias"
    alias tenter="terrainium enter --biome example_biome2"
    alias texit="terrainium exit"
    # USER DEFINED ALIASES: END
    # USER DEFINED ENVS: START
    export EDITOR="nano"
    export NEW="VALUE"
    export PAGER="less"
    # USER DEFINED ENVS: END
}

function terrainium_shell_constructor() {
    if [ "$TERRAIN_ENABLED" = "true" ]; then
        /bin/echo entering terrain
    fi
}

function terrainium_shell_destructor() {
    if [ "$TERRAIN_ENABLED" = "true" ]; then
        /bin/echo exiting terrain
    fi
}

function terrainium_enter() {
    terrainium_shell_constructor
}

function terrainium_exit() {
    if [ "$TERRAIN_ENABLED" = "true" ]; then
        builtin exit
    fi
}

function terrainium_preexec_functions() {
    tenter="(\$TERRAINIUM_EXECUTABLE enter*|$TERRAINIUM_EXECUTABLE enter*|*terrainium enter*)"
    texit="(\$TERRAINIUM_EXECUTABLE exit*|$TERRAINIUM_EXECUTABLE exit*|*terrainium exit*)"
    tconstruct="(\$TERRAINIUM_EXECUTABLE construct*|$TERRAINIUM_EXECUTABLE construct*|*terrainium construct*)"
    tdestruct="(\$TERRAINIUM_EXECUTABLE destruct*|$TERRAINIUM_EXECUTABLE destruct*|*terrainium destruct*)"

    if [ $TERRAIN_ENABLED = "true" ]; then
        case "$3" in
        $~texit)
            terrainium_exit
        ;;
        $~tconstruct)
            terrainium_shell_constructor
        ;;
        $~tdestruct)
            terrainium_shell_destructor
        ;;
        esac
    fi
}

function terrainium_zshexit_functions() {
    terrainium_shell_destructor
    "$TERRAINIUM_EXECUTABLE" exit
}

preexec_functions=(terrainium_preexec_functions $preexec_functions)
zshexit_functions=(terrainium_zshexit_functions $zshexit_functions)

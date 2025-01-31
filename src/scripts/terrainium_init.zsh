#!/usr/bin/env zsh

function __terrainium_auto_apply() {
    if [ -z "$TERRAINIUM_EXECUTABLE" ]; then
        TERRAINIUM_EXECUTABLE=terrainium
    fi

    auto_apply="$($TERRAINIUM_EXECUTABLE get --auto-apply 2> /dev/null)"
    if [ $? != 0 ]; then
        auto_apply="off"
    fi

    if [ "$auto_apply" = "enabled" ] || [ "$auto_apply" = "background" ]; then
        "$TERRAINIUM_EXECUTABLE" enter --auto-apply
    elif [ "$auto_apply" = "replaced" ] || [ "$auto_apply" = "all" ]; then
        exec "$TERRAINIUM_EXECUTABLE" enter --auto-apply
    fi
}

function __terrainium_chpwd_functions() {
    __terrainium_auto_apply
}

if [ "$TERRAIN_ENABLED" = "true" ]; then
    autoload -Uzw "${TERRAIN_INIT_SCRIPT}"
    "${terrain_init}"
    builtin unfunction -- "${terrain_init}"
    __terrainium_enter
else
    chpwd_functions=(__terrainium_chpwd_functions $chpwd_functions)
    __terrainium_auto_apply
fi

#!/usr/bin/env zsh

if [ -z "$TERRAINIUM_EXECUTABLE" ]; then
    TERRAINIUM_EXECUTABLE=terrainium
fi

if [ "$TERRAIN_ENABLED" = "true" ]; then
    autoload -Uzw "${TERRAIN_INIT_SCRIPT}"
    "${terrain_init}"
    builtin unfunction -- "${terrain_init}"
    terrainium_enter
fi

function terrainium_chpwd_functions() {
    if [ "$TERRAIN_ENABLED" != "true" ]; then
        auto_apply="$($TERRAINIUM_EXECUTABLE get --auto-apply 2> /dev/null)"
        if [ $? != 0 ]; then
            auto_apply="off"
        fi
        if [ "$auto_apply" = "enabled" ] || [ "$auto_apply" = "background" ]; then
            "$TERRAINIUM_EXECUTABLE" enter --auto-apply
        elif [ "$auto_apply" = "replaced" ] || [ "$auto_apply" = "all" ]; then
            exec "$TERRAINIUM_EXECUTABLE" enter --auto-apply
        fi
    fi
}

chpwd_functions=(terrainium_chpwd_functions $chpwd_functions)

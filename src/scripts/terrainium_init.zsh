#!/usr/bin/env zsh

function __terrainium_auto_apply() {
    auto_apply="$(terrainium get --auto-apply 2> /dev/null)"
    if [ $? != 0 ]; then
        auto_apply="off"
    fi

    if [ "$auto_apply" = "enabled" ] || [ "$auto_apply" = "background" ]; then
        terrainium enter --auto-apply
    elif [ "$auto_apply" = "replace" ] || [ "$auto_apply" = "all" ]; then
        exec terrainium enter --auto-apply
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
    # unexport but set terrainium env vars
    typeset +x TERRAIN_ENABLED
    typeset +x TERRAIN_SESSION_ID
    typeset +x TERRAIN_SELECTED_BIOME
    typeset +x TERRAIN_AUTO_APPLY
    typeset +x TERRAIN_DIR
    typeset +x TERRAIN_INIT_SCRIPT
    typeset +x terrain_init
else
    chpwd_functions=(__terrainium_chpwd_functions $chpwd_functions)
    __terrainium_auto_apply
fi

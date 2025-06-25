#!/usr/bin/env zsh

function __terrainium_auto_apply() {
    auto_apply="$(terrainium get --auto-apply 2> /dev/null)"
    if [ $? != 0 ]; then
        auto_apply="off"
    fi

    typeset -x FPATH
    if [ "$auto_apply" = "enabled" ] || [ "$auto_apply" = "background" ]; then
        terrainium enter --auto-apply
    elif [ "$auto_apply" = "replace" ] || [ "$auto_apply" = "all" ]; then
        exec terrainium enter --auto-apply
    fi
    typeset +x FPATH
}

function __terrainium_parse_command() {
    local command=(${(s/ /)1})
    if [ "${command[1]}" = "terrainium" ]; then
        typeset +x __terrainium_is_terrainium="true"
        typeset +x __terrainium_verb="${command[2]}"
    fi
}

function __terrainium_reexport_envs() {
    if [ -n "$FPATH" ]; then typeset -x FPATH; fi
    if [ -n "$TERRAIN_NAME" ]; then typeset -x TERRAIN_NAME; fi
    if [ -n "$TERRAIN_SESSION_ID" ]; then typeset -x TERRAIN_SESSION_ID; fi
    if [ -n "$TERRAIN_SELECTED_BIOME" ]; then typeset -x TERRAIN_SELECTED_BIOME; fi
    if [ -n "$TERRAIN_AUTO_APPLY" ]; then typeset -x TERRAIN_AUTO_APPLY; fi
    if [ -n "$TERRAIN_DIR" ]; then typeset -x TERRAIN_DIR; fi
    typeset +x __TERRAIN_ENVS_EXPORTED="true"
}

function __terrainium_unexport_envs() {
    # unexport but set terrainium env vars
    if [ -n "$FPATH" ]; then typeset +x FPATH; fi
    if [ -n "$TERRAIN_NAME" ]; then typeset +x TERRAIN_NAME; fi
    if [ -n "$TERRAIN_SESSION_ID" ]; then typeset +x TERRAIN_SESSION_ID; fi
    if [ -n "$TERRAIN_SELECTED_BIOME" ]; then typeset +x TERRAIN_SELECTED_BIOME; fi
    if [ -n "$TERRAIN_AUTO_APPLY" ]; then typeset +x TERRAIN_AUTO_APPLY; fi
    if [ -n "$TERRAIN_DIR" ]; then typeset +x TERRAIN_DIR; fi
    unset __TERRAIN_ENVS_EXPORTED
}

function __terrainium_fpath_preexec_function() {
    __terrainium_parse_command "$3"
    if [ "$__terrainium_is_terrainium" = "true" ]; then
        typeset -x FPATH
    fi
}

function __terrainium_fpath_precmd_function() {
    if [ "$__terrainium_is_terrainium" = "true" ]; then
        typeset +x FPATH
        unset __terrainium_is_terrainium
        unset __terrainium_verb
    fi
}

function __terrainium_chpwd_functions() {
    __terrainium_auto_apply
}

if [ -n "$TERRAIN_SESSION_ID" ]; then
    autoload -Uzw "${TERRAIN_INIT_SCRIPT}"
    "${terrain_init}"
    builtin unfunction -- "${terrain_init}"
    __terrainium_enter
    __terrainium_unexport_envs
    unset TERRAIN_INIT_SCRIPT
    unset terrain_init
else
    preexec_functions=(__terrainium_fpath_preexec_function $preexec_functions)
    precmd_functions=(__terrainium_fpath_precmd_function $precmd_functions)
    chpwd_functions=(__terrainium_chpwd_functions $chpwd_functions)
    __terrainium_auto_apply
fi

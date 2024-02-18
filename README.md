# terrainium

A command-line utility written in Rust for env management

- `terrainium` is cli utility that takes in a `toml` file as input and creates your
  development environment for you.
- This utility will automatically set environment variables, aliases and run specified
  commands in shell or in background.
- The sample configuration file is stored in [terrain.toml](./example_configs/terrain.full.toml)
- Currently only `zsh` is supported.

## Command-Line Arguments (Usage)

```sh
terrainium <verb> [OPTIONS]
```

- Verbs:

  - `init [OPTIONS]` - Generates terrain.toml in current directory or
    central location.

    - `-c|--central` - Stores terrain in `$XDG_CONFIG_HOME/terrainium/terrains/[...parent_]$(pwd)/terrain.toml`.
    - `-f|--full` - Generates example terrain with all possible options.
    - `-e|--edit` - Generates terrain and opens file in `EDITOR`.

  - `edit` - edit terrain with editor specified in `EDITOR` environment variable.

  - `update OPTIONS` - Updates terrain with options

    - `-s|--set-biome <name>` - set default `biome`. Cannot be used with other options.
    - `-b|--biome <biome_value>` - biome to update. Updates default if `default`
      is used. Updates terrain if `none` is used. Cannot be used with `-n` flag.
    - `-n|--new <new>` creates a new biome with `name`. If `-e`|`-a` are passed with
      this, the environment variable and alias will be set for the new biome.
      Cannot be used with `-b` flag.
    - `-e|--env <VAR_NAME>=<VAR_VALUE>` adds or updates environment variable `VAR_NAME`
      with value `VAR_VALUE`.
    - `-a|--alias <ALIAS_NAME>=<ALIAS_VALUE>` adds or updates alias `ALIAS_NAME`
      with value `ALIAS_VALUE`.
    - `-k|--backup` creates a backup terrain.toml in the same directory before
      updating the original.

  - `generate` - generates and compiles required scripts.

  - `get [OPTIONS]` - Get the values that will be applied. If no options passed
    will return all values.

    - `-b|--biome <name>` - name of the biome for which values to be retrieved.
      `default` to use default biome. `none` to remove biome get values from terrain.
    - `--alias` - returns value of all aliases defined.
    - `--env` - returns value of all environment variables defined.
    - `-e [name]` - returns value of environment variable with `name`.
    - `-a [name]` - returns value of alias with `name`.
      will return value of that specific alias.
    - `-c|--constructors` - returns value of constructors defined.
    - `-d|--destructors` - returns value of constructors defined.

  - `enter [OPTIONS]` - applies terrain.

    - `-b|--biome <name>` - name of the biome to be applied. `default` to use
      default biome. `none` to only use main terrain without biome.

  - `exit` - exits terrain.

  - `construct [OPTIONS]` - runs commands specified in construct block.

    - `-b|--biome <name>` - name of the biome to be used. Values can be same as
      `terrainium enter`.

  - `deconstruct [OPTIONS]` - runs commands specified in destructor block.

    - `-b|--biome <name>` - name of the biome to be used. Values can be same as
      `terrainium enter`.

  - `-h|--help` - shows help.

## Shell Integration

### zsh

- For zsh add this to your `.zshrc`

```sh
if [ "$TERRAINIUM_ENABLED" = "true" ];then
    autoload -Uzw "${TERRAINIUM_INIT_FILE}"
    "${TERRAINIUM_INIT_ZSH}"
    builtin unfunction -- "${TERRAINIUM_INIT_ZSH}"
    terrainium_enter
fi
```

## What does it do?

- A `terrain` is a complete environment with environment variables, aliases, and
  commands to spawn as foreground and background processes.
- A `biome` has similar structure to a `terrain`, but it will be applied on top
  of the `terrain`.
- When user runs `terrainium enter` command without options following things will
  happen:
  1. Without any arguments if `default-biome` is set in `terrain.toml`,
     `terrainium` will:
     1. combine environment variables from `default-biome` and main `terrain`.
        If there are any environment variables redefined in biome they will take
        precedence.
     1. combine aliases from `default-biome` and main `terrain`.
        If there are any aliases redefined in biome they will take precedence.
     1. Specified constructors will be merged from `default-biome` and `terrain`.
     1. Specified destructors will be merged from `default-biome` and `terrain`.
     1. start a shell with merged environment variables and aliases.
     1. run foreground processes in shell created in above steps.
     1. start running background processes.
  1. If `-b <biome_name>` is passed:
     1. if `biome_name` is name of the biome defined, the sub-steps in step 1 will
        be done but for specified biome name.
     1. if `biome_name` is `default` will do same as step 1.
     1. if `biome_name` is `none` will not merge with any biome and start `terrain`
        as is.

### Example

- If [terrain.toml](./example_configs/terrain.full.toml) is used.
- When `terrainium enter` is run, the default biome `example_biome` will be applied:

  1. environment variables set will be:
     1. `EDITOR=nvim` -- from `example_biome`
     1. `TEST=value` -- from `terrain`
  1. aliases set will be:
     1. `tedit=terrainium edit` -- from `terrain`
     1. `tenter=terrainium enter -b example_biome` -- from `example_biome`
  1. In shell that was started, following commands will be run:
     1. `echo entering terrain` -- from `terrain`
     1. `echo entering example_biome` -- from `example_biome`
  1. In background following commands will be run:
     1. `run something` -- from terrain.

- When `terrainium exit` is run, the terrain will closed:
  1. In shell that was started by `terrainium enter` following commands will be run:
     1. `echo exiting terrain` -- from `terrain`
     1. `echo exiting example_biome` -- from `example_biome`
  1. In background following commands will be run:
     1. `stop something` -- from terrain.
  1. The shell started by `terrainium enter` will be closed.

### constructors and destructors

- When `construct` or `deconstruct` command is run, 2 types of processes are spawned.

1. `foreground` - Which are run in shell activated by `terrainium enter` command.
2. `background` - which are separate processes started in background and logs for
   these processes are present in `/tmp/terrainium-$TERRAINIUM_SESSION_ID/` directory.

- When terrain is activated `TERRAINIUM_SESSION_ID` variable is set, it is an UUID
  and changes every session. The STDOUT and STDERR logs of spawned processes are
  stored in `/tmp/terrainium-$TERRAINIUM_SESSION_ID/` directory.

- created files will be named similar to following:

```files
$ ls /tmp/terrainium-94e30640-eaaf-4c4e-9db9-950db177044b/
spawn-err-6d11dd42-b88f-4c1a-82b8-4171e0d2fd09.log      # ->  # STDERR of terrainium_executor using which process is started
spawn-out-6d11dd42-b88f-4c1a-82b8-4171e0d2fd09.log      # ->  # STDOUT of terrainium_executor using which process is started
std_out-6d11dd42-b88f-4c1a-82b8-4171e0d2fd09.log        # ->  # STDOUT of process
std_err-6d11dd42-b88f-4c1a-82b8-4171e0d2fd09.log        # ->  # STDERR of process
status-6d11dd42-b88f-4c1a-82b8-4171e0d2fd09.json        # ->  # status of process include command, args and exit code
```

- In above list of files `6d11dd42-b88f-4c1a-82b8-4171e0d2fd09`
  is UUID for specific process started using `terrainium_executor`. i.e.
  logs are for single background process.
- And `94e30640-eaaf-4c4e-9db9-950db177044b` is `TERRAINIUM_SESSION_ID`.

## For developers

- If `TERRAINIUM_DEV` is set to `true`, `terrainium` and `terrainium_executor`
  binaries will be use from debug directory.

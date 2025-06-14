# terrainium

A command-line utility written in Rust for env management

- `terrainium` is cli utility that takes in a `terrain.toml` file as input and
  creates your development environment for you.
- This utility will automatically set environment variables, aliases and run specified
  commands in shell or in background.
- The sample configuration file is stored in [terrain.toml](./tests/data/terrain.example.toml)
- Currently only `zsh` is supported.

## Command-Line Arguments (Usage)

### terrainium

```sh
terrainium <verb|OPTIONS> [OPTIONS]
```

- Verbs:

    - `init [OPTIONS]` - Generates terrain.toml in current directory or
      central location.

        - `-c|--central` - Stores terrain in `~/.config/terrainium/terrains/[..._parent]_$(pwd)/terrain.toml`.  
          e.g. if terrain is defined in directory `/home/user/work/repos/terrainium` the `terrain.toml` will be stored
          in `/home/user/.config/terrainium/terrains/_home_user_work_repos_terrainium/terrain.toml`.
        - `-x|--example` - Generates example terrain with all possible options.
        - `-e|--edit` - Generates terrain and opens file in `EDITOR`.

    - `edit` - edit terrain with editor specified in `EDITOR` environment variable.

    - `update OPTIONS` - Updates terrain with options

        - `-s|--set-default <name>` - set default `biome`.  
          _Cannot be used with other options._
        - `-b|--biome <biome_name>` - biome to update.  
          _Cannot be used with `-n` flag._
        - `-n|--new <new>` creates a new biome with `name`. If `-e`|`-a` are passed with
          this, the environment variable and alias will be set for the new biome.  
          _Cannot be used with `-b` flag._
        - `-e|--env <VAR_NAME>=<VAR_VALUE>` adds or updates environment variable `VAR_NAME`
          with value `VAR_VALUE`. Only single variable can be passed with a single `-e`.
          i.e. if we want to pass 2 variables user will have to do following:

          ```shell
          terrainium update -e NEW_VAR1=VALUE1 -e NEW_VAR2=VALUE2
          ```

        - `-a|--alias <ALIAS_NAME>=<ALIAS_VALUE>` adds or updates alias `ALIAS_NAME`
          with value `ALIAS_VALUE`. Only single alias can be passed with a single `-a`.
          i.e. if we want to pass 2 aliases user will have to do following:

          ```shell
          terrainium update -a new_alias1=value1 -a new_alias2=value2
          ```

        - `--auto-apply <AUTO_APPLY_VALUE>` update auto-apply config. Value can be
          `all`, `enabled`, `background`, `replace`, `off`.
        - `-k|--backup` creates a backup `terrain.toml.bkp` in the same directory before
          updating the original.

    - `generate` - generates and compiles required shell scripts.

    - `validate` - validates the `terrain.toml` and shows error and warnings if any.

    - `get [OPTIONS]` - Get the values that will be applied. If no options passed
      will return all values.

        - `-b|--biome <biome_name>` - name of the biome for which values to be retrieved.
        - `--aliases` - returns value of all aliases defined.
        - `--envs` - returns value of all environment variables defined.
        - `-e|--env [name]` - returns value of environment variable with `name`.
        - `-a|--alias [name]` - returns value of alias with `name`.
        - `-c|--constructors` - returns value of all the constructors defined.
        - `-d|--destructors` - returns value of all the destructors defined.
        - `--auto-apply` - returns auto apply config value.
          output will be one of `all`, `enabled`, `background`, `replace`, `off`.
        - `--debug` - by default this command does not print any terrain validation
          logs for automation purpose. pass this flag to print them.

    - `enter [OPTIONS]` - applies terrain.

        - `-b|--biome <biome_name>` - name of the biome to be applied.

    - `exit` - exits terrain.

    - `construct [OPTIONS]` - runs commands specified in constructor block.

        - `-b|--biome <biome_name>` - name of the biome for which constructors are run.

    - `destruct [OPTIONS]` - runs commands specified in destructor block.

        - `-b|--biome <biome_name>` - name of the biome for which destructors are run.

    - `status [OPTIONS]` - fetches the status of the currently applied or
      recent session of the terrain from the daemon.

        - `-j|--json` - print status in json format.
        - `-r|--recent <n>` - fetches status of last `n`th session.
        - `-s|--session-id <session_id>` - specify session for which status is to be fetched.

    - `-h|--help` - shows help.

    - **NOTE** - The `-b|--biome <biome_name>` argument for all supported commands following is the behavior:
        - if not specified `default-biome` will be selected.
        - if there is no `default-biome` main terrain will be used.
        - if value `none` is passed main terrain will be used regardless of other biome definitions.
        - if biome with name `biome_name` is not present error will be thrown.

- Options:
    - `--create-config` - creates a config file at location:
      `~/.config/terrainium/terrainium.toml`.
    - `--update-rc` - to update `~/.zshrc` to source init script
    - `--update-rc-path <path>` - to update `<path>` to source init script
    - `-l|--log-level` - select log level to validation messages.
      Value can be `trace`, `debug`, `info`, `warn` and `error`.

### terrainiumd

```sh
terrainiumd [OPTIONS]
```

- Options:
    - `--create-config` - creates a config file at location:
      `~/.config/terrainium/terrainiumd.toml`.
    - `-f|--force` - remove existing unix socket and start daemon
    - `-l|--log-level` - select log level of daemon. Value can be `trace`,
      `debug`, `info`, `warn` and `error`.
    - `-h|--help` - shows help.

## What does it do?

- A `terrain` is a complete environment with environment variables, aliases, and
  commands to spawn as foreground and background processes.
- A `biome` has similar structure to a `terrain`, and provides way of customizing
  terrain behavior.
- When user runs `terrainium enter` command without options following things will
  happen:
    - Without any arguments if `default-biome` is set in `terrain.toml`,
      `terrainium` will:
        1. combine the environment variables from `default-biome` and the main `terrain`.
           if there are any environment variables redefined in biome they will take
           precedence.
        2. combine the aliases from `default-biome` and main `terrain`.
           if there are any aliases redefined in biome they will take precedence.
        3. specified constructors will be merged from `default-biome` and `terrain`.
           constructors from `terrain` will run first, then the constructors of
           `default-biome`.
        4. start a shell with merged environment variables and aliases.
        5. run foreground processes in shell created in above steps.
        6. start running background processes in `terrainiumd` process.

    - If `-b <biome_name>` is passed:
        - biome will be selected based on `biome_name` (sames behavior as defined above).
        - follow steps mentioned above for selected biome instead of `default-biome`

### Auto Apply

- If `auto-apply` is enabled in the config, and when directory is changed using
  `cd` command. The `default-biome` will be selected and `terrainium enter` will
  run with following conditions:

    1. If only `enabled` is true only the background commands will **NOT** run.
    2. If `enabled` and `background` is true, even background commands will run.
    3. If `enabled` and `replace` is set as true, then again background commands
       will **NOT** be run. But newly spawned shell will replace your
       existing shell. For more information look into `exec` command in shells.
    4. If all `enabled`, `background` and `replace` are true entire terrain will be
       applied automatically, and terrainium shell will become top process.

### Constructors and Destructors

- When `construct` or `destruct` command is run, 2 types of processes are spawned:

    1. `foreground` - Which are run in shell activated by `terrainium enter` command.
    2. `background` - which are separate processes started in background and logs for
       these processes are present in `/tmp/terrainiumd/terrains/<terrain_name>/$TERRAINIUM_SESSION_ID/` directory.

- When terrain is activated `TERRAINIUM_SESSION_ID` environment variable is set,
  it is a UUID and changes every session.
- `cwd` can be specified in terrain commands definition to run command in that
  specified directory. `cwd` can be absolute or relative to terrain directory.
  If `cwd` is not specified terrain directory will be used to run commands.

### Terrainium Daemon

- `terrainiumd` is a daemon process / service that runs all background commands
  (constructors and destructors) defined in the terrain.
- The daemon will run using unix socket, the socket will be created at
  `/tmp/terrainiumd/socket`.
- The daemon will receive request from client (`terrainium`),
  and will execute background constructors and destructor.
- It also stores terrain session status in `/tmp/terrainiumd/<terrain_name>/<session_id>`
  directory.
- When `terrainium status` command is run daemon will return the status stored in
  `/tmp/terrainiumd/<terrain_name>/<session_id>/state.json` file
- The logs for background commands will be stored in status directory in following
  pattern: `<constructors|destructors>.<index>.<timestamp>.log`  
  where: index is based on commands specified in `terrain.toml`.

### Example

- If [terrain.toml](./tests/data/terrain.example.toml) is used.
- When `terrainium enter` is run, the default biome `example_biome` will be applied:
    1. environment variables set will be:
        - `EDITOR=nvim` -- from `example_biome`
        - `ENV_VAR="overridden_env_val"` -- from `example_biome`
        - `POINTER_ENV_VAR="overridden_env_val"`  
          -- value defined in definition is `"${ENV_VAR}"`. The value of `ENV_VAR`
          will be substituted from selected biome.
        - `NESTED_POINTER="overridden_env_val-overridden_env_val-${NULL}"`  
          -- same as above but as `NULL_POINTER` will be replaced with `${NULL}`.
          if `NULL` env var is set in parent shell it will be substituted here.
        - `PAGER="less"` -- from terrain
        - `NULL_POINTER=${NULL}` -- from terrain
        - `TERRAIN_DIR=<terrain directory>` -- terrainium env var, always added
        - `TERRAIN_SELECTED_BIOME="example_biome"` -- terrainium env var, always added
        - `TERRAIN_SESSION_ID=<uuid>` -- terrainium env var, always added
    2. aliases set will be:
        - `texit=terrainium exit` -- from `terrain`
        - `tenter=terrainium enter -b example_biome` -- from `example_biome`
    3. In shell that was started, following commands will be run:
        - `/bin/echo entering terrain` -- from `terrain` inside terrain directory as
          `cwd` is not specified
        - `/bin/echo entering example_biome` -- from `example_biome` inside terrain
          directory as `cwd` is not specified
    4. In background following commands will be run:
        - `/bin/bash -c ${PWD}/tests/scripts/print_num_for_10_sec` -- from
          `example_biome`.

- When `terrainium exit` is run, the terrain will be closed:
    1. In shell that was started by `terrainium enter` following commands will be run:
        1. `/bin/echo exiting terrain` -- from `terrain`
        2. `/bin/echo exiting example_biome` -- from `example_biome`
    2. In background following commands will be run:
        1. `/bin/bash -c ${TERRAIN_DIR}/tests/scripts/print_num_for_10_sec` -- from
           `example_biome`.
        2. The shell started by `terrainium enter` will be closed.

## Shell Integration

### zsh

- For zsh add this to your `~/.zshrc`

```sh
source "$HOME/.config/terrainium/shell_integration/terrainium_init.zsh"
```

- You can also do this to using `terrainium --update-rc` command.
- If you want to update file different from `~/.zshrc` use `terrainium --update-rc-path <path>`

## For developers

- If `TERRAINIUM_DEV` is set to `true` in the terrain, `terrainium` and `terrainiumd`
  binaries will be used from debug directory.
- Feature `terrain-schema` can be used to generate json schema for `terrain.toml`.
- `schema` argument can be passed to `terrainium` command when built with
  `terrain-schema` feature that will generate required json schemas in directory
  [`./schema/`](./schema).

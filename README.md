# terrainium

A command-line utility written in Rust for env management

- **terrainium** is CLI utility that takes in a `terrain.toml` file as input and
  creates your development environment for you.
- This utility will automatically set environment variables, aliases and run specified
  commands in shell or in background.
- The sample configuration file is stored in [terrain.toml](./example_configs/terrain.example.toml)
- Currently only `zsh` is supported.
- Information about `terrain.toml` files anatomy can be found in [TERRAIN.md](./docs/TERRAIN.md).
- Information about configuration options for behavior of `terrain` and `terrainiumd` commands
  can be found in [CONFIGS.md](./docs/CONFIGS.md).

## Command-Line Arguments (Usage)

### terrain

```sh
terrain <COMMAND|OPTIONS> [OPTIONS]
```

- Commands:
  - `init [OPTIONS]` - Generates `terrain.toml` in current directory or
    central location.
    - `-c|--central` - Stores terrain in central directory, e.g. if terrain is
      defined for project in directory `/home/user/work/repos/terrainium`, then
      the `terrain.toml` will be created in `/home/user/.config/terrainium/terrains/_home_user_work_repos_terrainium/terrain.toml`.
    - `-x|--example` - Generates example terrain with all possible options.
    - `-e|--edit` - Generates terrain and opens file in `EDITOR`.

  - `edit [OPTIONS]` - edit current directory's terrain with editor specified in
    `EDITOR` environment variable.
    - `--active` - opens editor for active terrain rather than current directory

  - `update OPTIONS` - Updates terrain with options
    - `-s|--set-default <DEFAULT>` - set default `biome`.  
      _Cannot be used with other options._
    - `-b|--biome <BIOME>` - biome to update.  
      _Cannot be used with `-n` flag._
    - `-n|--new <NEW>` creates a new biome with `name`. If `-e`|`-a` are passed with
      this, the environment variable and alias will be set for the new biome.  
      _Cannot be used with `-b` flag._
    - `-e|--env <VAR_NAME>=<VAR_VALUE>` adds or updates environment variable `VAR_NAME`
      with value `VAR_VALUE`. If value has space double quotes can be used `-e VAR="SOME VALUE"`.  
      Only single variable can be passed with a single `-e`.
      i.e. If we want to pass 2 variables user will have to do following:

      ```shell
      terrain update -e NEW_VAR1=VALUE1 -e NEW_VAR2=VALUE2
      ```

    - `-a|--alias <ALIAS_NAME>=<ALIAS_VALUE>` adds or updates alias `ALIAS_NAME`
      with value `ALIAS_VALUE`.If value has space, double quotes can be used `-a alias="SOME VALUE"`.  
      Only single alias can be passed with a single `-a`.
      i.e. If we want to pass 2 aliases user will have to do following:

      ```shell
      terrain update -a new_alias1=value1 -a new_alias2=value2
      ```

    - `--auto-apply <AUTO_APPLY_VALUE>` update auto-apply configuration.
      Value can be `all`, `enabled`, `background`, `replace`, `off`.
    - `--backup` creates a backup `terrain.toml.bkp` in the same directory before
      updating the original.
    - `--active` updates active terrain rather than current directory

  - `generate [OPTIONS]` - generates and compiles required shell scripts.  
    **Must** be executed if terrain.toml is updated commands other
    than `terrain edit`, `terrain update`
    - `--active` generates for active terrain rather than current directory

  - `validate [OPTIONS]` - validates the `terrain.toml` and shows error and warnings if any.
    - `--active` validates the active terrain rather than current directory

  - `get [OPTIONS]` - Get the values that will be applied. If no options passed
    will return all values.
    - `-b|--biome <BIOME>` - name of the biome for which values to be retrieved.
    - `-e|--env <ENV>` - returns value of environment variable with `name`.
    - `-a|--alias <ALIAS>` - returns value of alias with `name`.
    - `-c|--constructors` - returns value of all the constructors defined.
    - `-d|--destructors` - returns value of all the destructors defined.
    - `--envs` - returns value of all environment variables defined.
    - `--aliases` - returns value of all aliases defined.
    - `--auto-apply` - returns auto apply configuration value.
      Output will be one of `all`, `enabled`, `background`, `replace`, `off`.
    - `-j|--json` - get all values in json format.
      _Cannot be used with options other than `--active`, `--debug`_
    - `--active` fetches the active terrain rather than current directory
    - `--debug` - by default this command does not print any terrain validation
      logs for automation purpose. Pass this flag to print them.

  - `enter [OPTIONS]` - applies terrain.
    - `-b|--biome <BIOME>` - name of the biome to be applied.

  - `exit` - exits terrain.

  - `construct [OPTIONS]` - runs commands specified in constructor block.
    - `-b|--biome <BIOME>` - name of the biome for which constructors are run.

  - `destruct [OPTIONS]` - runs commands specified in destructor block.
    - `-b|--biome <BIOME>` - name of the biome for which destructors are run.

  - `status [OPTIONS]` - fetches the status of the currently applied or
    recent session of the terrain from the daemon.
    - `-t|--terrain-name` - name of the terrain for which status is to be fetched.
    - `-j|--json` - print status in `json` format.
    - `-r|--recent <N>` - fetches status of last `N`th session.
    - `-s|--session-id <SESSION_ID>` - specify session for which status is to be fetched.

  - `-h|--help` - shows help.

  - **NOTE**
    - The `-b|--biome <BIOME>` argument for all supported commands following is the behavior:
      - if not specified `default-biome` will be selected.
      - if there is no `default-biome`, then main terrain will be used.
      - if value `none` is passed main terrain will be used regardless of other biome definitions.
      - if biome with name `biome_name` is not present error will be thrown.
    - The `--active` argument is useful when terrainium shell is active, but user is in
      a different directory that has terrain defined.
      - The commands that provide `--active` flag by default try to use terrain for
        current directory rather than activated terrain in current shell session.

- Options:
  - `--create-config` - creates a configuration file at location:
    `~/.config/terrainium/terrainium.toml`.
    _Cannot be used with other options._
  - `--update-rc [path]` - update `path` if specified or `~/.zshrc` to source shell integration script.
  - `-l | --log-level` - select log level to validation messages.
    Can be used with subcommands as well.
    Value can be `trace`, `debug`, `info`, `warn` and `error`.

### terrainiumd

```sh
terrainiumd <COMMAND|OPTIONS> [OPTIONS]
```

- Commands:
  - `install` - installs the `terrainiumd` as a service and enables, starts the installed service.
  - `remove` - removes the `terrainiumd` as a service and stops the installed service.
  - `enable [OPTION]` - enables `terrainiumd` service to be started on the machine startup.
    - `-n|--now` - starts the service after enabling it.
  - `disable [OPTION]` - disables `terrainiumd` service to be started on the machine startup.
    - `-n|--now` - stops the service after disabling it.
  - `start` - start the `terrainiumd` process now if not running.
  - `stop` - stop the `terrainiumd` process now if running.
  - `reload` - just reloads the service in the system (`launchd`, `systemd`).
    Does NOT start the service.
  - `status` - prints status of the installed service, status can be: `running(enabled|disabled)`,
    `not running(enabled|disabled)`, `not loaded`, `not installed`

- Options:
  - `--run` - starts the terrainium daemon
  - `--create-config` - creates a configuration file at location:
    `~/.config/terrainium/terrainiumd.toml`.
  - `-f|--force` - remove existing Unix socket and start daemon
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
    `terrain` will:
    1. combine the environment variables from `default-biome` and the main `terrain`.
       If there are any environment variables redefined in biome they will take
       precedence.
    2. combine the aliases from `default-biome` and main `terrain`.
       If there are any aliases redefined in biome they will take precedence.
    3. specified constructors will be merged from `default-biome` and `terrain`.
       Constructors from `terrain` will run first, then the constructors of
       `default-biome`.
    4. start a shell with merged environment variables and aliases.
    5. run foreground processes in shell created in above steps.
    6. start running background processes in `terrainiumd` process.

  - If `-b <biome_name>` is passed:
    - biome will be selected based on `biome_name` (sames behavior as defined above).
    - follow steps mentioned above for selected biome instead of `default-biome`

### Auto Apply

- If `auto-apply` is enabled in the configuration, and when directory is changed using
  `cd` command. The `default-biome` will be selected and `terrain enter` will
  run with following conditions:
  1. If only `enabled` is true only the background commands will **NOT** run.
  2. If `enabled` and `background` is true, even background commands will run.
  3. If `enabled` and `replace` is set as true, then again background commands
     will **NOT** be run. But newly spawned shell will replace your
     existing shell. For more information look into `exec` command in shells.
  4. If all `enabled`, `background` and `replace` are true entire terrain will be
     applied automatically, and `terrainium` shell will become top process.

### Constructors and Destructors

- When `construct` or `destruct` command is run, 2 types of processes are spawned:
  1. `foreground` - Which are run in shell activated by `terrain enter` command.
  2. `background` - which are separate processes started in background and logs for
     these processes are present in `/tmp/terrainiumd/terrains/<terrain_name>/$TERRAINIUM_SESSION_ID/` directory.

- When terrain is activated `TERRAINIUM_SESSION_ID` environment variable is set,
  it is a `UUID` and changes every session.
- `cwd` can be specified in terrain commands definition to run command in that
  specified directory. `cwd` can be absolute or relative to terrain directory.
  If `cwd` is not specified terrain directory will be used to run commands.

### Terrainium Daemon

- `terrainiumd` is a daemon process / service that runs all background commands
  (constructors and destructors) defined in the terrain.
- The daemon will run using Unix socket, the socket will be created at
  `/tmp/terrainiumd/socket`.
- The daemon will receive request from client (`terrain`),
  and will execute background constructors and destructor.
- It also stores terrain session status in `/tmp/terrainiumd/<terrain_name>/<session_id>`
  directory.
- When `terrain status` command is run daemon will return the status stored in
  `/tmp/terrainiumd/<terrain_name>/<session_id>/state.json` file
- The logs for background commands will be stored in status directory in following
  pattern: `<constructors|destructors>.<index>.<timestamp>.log`  
  where: index is based on commands specified in `terrain.toml`.
- It can also be installed as `launchd` or `systemd` service on macOS and linux
  respectively.

### Example

- If [terrain.toml](./example_configs/terrain.example.toml) is used.
- When `terrain enter` is run, the default biome `example_biome` will be applied:
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
     - environment variable `TERRAIN_DIR` is also set but not exported. i.e. This
       environment variable won't be available to the child processes of shell but can
       be used in shell itself.
       This can be used by user if they want to execute something in terrain directory.
  2. aliases set will be:
     - `texit=terrain exit` -- from `terrain`
     - `tenter=terrain enter -b example_biome` -- from `example_biome`
  3. In shell that was started, following commands will be run:
     - `/bin/echo entering terrain` -- from `terrain` inside terrain directory as
       `cwd` is not specified
     - `/bin/echo entering example_biome` -- from `example_biome` inside terrain
       directory as `cwd` is not specified
  4. In background following commands will be run:
     - `/bin/bash -c ${PWD}/tests/scripts/print_num_for_10_sec` -- from
       `example_biome`.

- When `terrain exit` is run, the terrain will be closed:
  1. In shell that was started by `terrain enter` following commands will be run:
     1. `/bin/echo exiting terrain` -- from `terrain`
     2. `/bin/echo exiting example_biome` -- from `example_biome`
  2. In background following commands will be run:
     1. `/bin/bash -c ${TERRAIN_DIR}/tests/scripts/print_num_for_10_sec` -- from
        `example_biome`.
     2. The shell started by `terrain enter` will be closed.

## Shell Integration

### zsh

- For zsh add this to your `~/.zshrc`

```sh
source "$HOME/.config/terrainium/shell_integration/terrainium_init.zsh"
```

- You can also do this to using `terrain --update-rc` command.
- If you want to update file different from `~/.zshrc` use `terrain --update-rc-path <path>`

## For developers

- If `TERRAINIUM_DEV` is set to `true` in the terrain, `terrain` and `terrainiumd`
  binaries will be used from debug directory.
- Feature `terrain-schema` can be used to generate json schema for `terrain.toml`.
- `schema` argument can be passed to `terrain` command when built with
  `terrain-schema` feature that will generate required json schemas in directory
  [`./schema/`](./schema).

# terrainium

A command-line utility written in Rust for env management

- `terrainium` is cli utility that takes in a `toml` file as input and creates your
  development environment for you.
- This utility will automatically set environment variables, aliases and run specified
  commands in shell or in background.
- The sample configuration file is stored in [terrain.toml](./tests/data/terrain.example.toml)
- Currently only `zsh` is supported.

## Command-Line Arguments (Usage)

```sh
terrainium <verb> [OPTIONS]
```

- Verbs:

    - `init [OPTIONS]` - Generates terrain.toml in current directory or
      central location.

        - `-c|--central` - Stores terrain in `$XDG_CONFIG_HOME/terrainium/terrains/[...parent_]$(pwd)/terrain.toml`.
        - `-x|--example` - Generates example terrain with all possible options.
        - `-e|--edit` - Generates terrain and opens file in `EDITOR`.

    - `edit` - edit terrain with editor specified in `EDITOR` environment variable.

    - `update OPTIONS` - Updates terrain with options

        - `-s|--set-default <name>` - set default `biome`.
          Cannot be used with other options.
        - `-b|--biome <biome_value>` - biome to update.
          Updates terrain if `none` is used. Will update currently active terrain
          if `current` is used. Cannot be used with `-n` flag.
        - `-n|--new <new>` creates a new biome with `name`. If `-e`|`-a` are passed with
          this, the environment variable and alias will be set for the new biome.
          Cannot be used with `-b` flag.
        - `-e|--env <VAR_NAME>=<VAR_VALUE>` adds or updates environment variable `VAR_NAME`
          with value `VAR_VALUE`.
        - `-a|--alias <ALIAS_NAME>=<ALIAS_VALUE>` adds or updates alias `ALIAS_NAME`
          with value `ALIAS_VALUE`.
        - `-k|--backup` creates a backup terrain.toml in the same directory before
          updating the original.
        - `--auto-apply <AUTO_APPLY_VALUE>` update auto-apply config. Value can be
          `all`, `enabled`, `replace`, `background`

    - `generate` - generates and compiles required scripts.

    - `get [OPTIONS]` - Get the values that will be applied. If no options passed
      will return all values.

        - `-b|--biome <name>` - name of the biome for which values to be retrieved.
          Gets main terrain if `none` is used. Will get values of currently active
          terrain if `current` is used.
        - `--alias` - returns value of all aliases defined.
        - `--env` - returns value of all environment variables defined.
        - `-e [name]` - returns value of environment variable with `name`.
        - `-a [name]` - returns value of alias with `name`.
          will return value of that specific alias.
        - `-c|--constructors` - returns value of constructors defined.
        - `-d|--destructors` - returns value of constructors defined.
        - `--auto-apply` - returns auto apply config value. `true` if enabled and
          `replace` if replace is enabled.

    - `enter [OPTIONS]` - applies terrain.

        - `-b|--biome <name>` - name of the biome to be applied. `default` to use
          default biome. `none` to only use main terrain without biome.

    - `exit` - exits terrain.

    - `construct` - runs commands specified in constructor block.

    - `destruct` - runs commands specified in destructor block.

    - `-h|--help` - shows help.

```sh
terrainiumd [OPTIONS]
```

- Options:
    - `-f|--force` - remove existing unix socket and start daemon
    - `-l|--log-level` - select log level of daemon. Value can be `trace`,
      `debug`, `info`, `warn` and `error`.
    - `-h|--help` - shows help.

## Shell Integration

### zsh

- For zsh add this to your `.zshrc`

```sh
if [ "$TERRAIN_ENABLED" = "true" ];then
    clear
    autoload -Uzw "${TERRAIN_INIT_SCRIPT}"
    "${terrain_init}"
    builtin unfunction -- "${terrainium_init}"
    terrainium_enter
    echo init....
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
    2. combine aliases from `default-biome` and main `terrain`.
       If there are any aliases redefined in biome they will take precedence.
    3. Specified constructors will be merged from `default-biome` and `terrain`.
    4. Specified destructors will be merged from `default-biome` and `terrain`.
    5. start a shell with merged environment variables and aliases.
    6. run foreground processes in shell created in above steps.
    7. start running background processes.
2. If `-b <biome_name>` is passed:
    1. if `biome_name` is name of the biome defined, the sub-steps in step 1 will
       be done but for specified biome name.
    2. if `biome_name` is `none` will not merge with any biome and start `terrain`
       as is.
3. If Auto Apply is enabled in config when directory is changed using `cd` command on zsh,
   terrain with default biome as selected will be automatically entered.
   i.e. similar to `terrainium enter` but following conditions are followed:

    1. If only `enabled` is true only env vars, aliases and foreground commands will be run. **NOT** background
       commands.
    2. If `enabled` and `replace` as above only env vars, aliases and foreground commands
       will be run. **NOT** background commands. But newly spawned shell will become
       top process for that terminal session. For more information look into `exec` command in shells.
    3. If all `enabled`, `background` and `replace` are true entire terrain will be
       applied automatically.

### Example

- If [terrain.toml](./tests/data/terrain.example.toml) is used.
- When `terrainium enter` is run, the default biome `example_biome` will be applied:

1. environment variables set will be:
    1. `EDITOR=nvim` -- from `example_biome`
    2. `TEST=value` -- from `terrain`
2. aliases set will be:
    1. `tedit=terrainium edit` -- from `terrain`
    2. `tenter=terrainium enter -b example_biome` -- from `example_biome`
3. In shell that was started, following commands will be run:
    1. `echo entering terrain` -- from `terrain`
    2. `echo entering example_biome` -- from `example_biome`
4. In background following commands will be run:
    1. `run something` -- from terrain.

- When `terrainium exit` is run, the terrain will be closed:

1. In shell that was started by `terrainium enter` following commands will be run:
    1. `echo exiting terrain` -- from `terrain`
    2. `echo exiting example_biome` -- from `example_biome`
2. In background following commands will be run:
    1. `stop something` -- from terrain.
3. The shell started by `terrainium enter` will be closed.

### constructors and destructors

- When `construct` or `destruct` command is run, 2 types of processes are spawned.

1. `foreground` - Which are run in shell activated by `terrainium enter` command.
2. `background` - which are separate processes started in background and logs for
   these processes are present in `/tmp/terrainiumd/terrains/<terrain_name>/$TERRAINIUM_SESSION_ID/` directory.

- When terrain is activated `TERRAINIUM_SESSION_ID` variable is set, it is a UUID
  and changes every session.

- **Note:** The paths used in background process must be absolute as they are run
  by daemon. You can use `$TERRAIN_DIR` environment variable to use paths from project
  without having to specify entire absolute path.

### Terrainium Daemon

- To run background process there is a daemon which will run in background.
- The daemon will run using unix socket, the socket will be created at `/tmp/terrainiumd/socket`.
- The daemon will receive request from client (`terrainium`) and will execute background
  constructors and destructor.

## For developers

- If `TERRAINIUM_DEV` is set to `true`, `terrainium` and `terrainium_executor`
  binaries will be used from debug directory.
- Feature `terrain-schema` can be used to generate json schema for `terrain.toml`.
  `schema` argument can be passed to `terrainium` command when built with
  `terrain-schema` feature that will generate a schema json at path `./schema/terrain-schema.json`.

# Anatomy of terrain.toml

- This document explains all the fields in [`terrain.toml`](../example_configs/terrain.example.toml) file.

## `$schema`

- JSON schema for `terrain.toml` file.
- Useful when LSP / schema validation is used in text editors for toml files.

## `name`

- Name of the terrain.
- Used by terrainiumd to store the status of background commands.
- Should be used with `terrain status -t <terrain_name>`.

## `auto_apply`

- Denotes how auto apply mechanism should behave.
- Auto apply mechanism defines what should happen when user starts a new shell
  inside terrain directory, or `cd`s into terrain directory.
- Possible values are: `all`, `replace`, `background`, `enabled`, `off`.
- When user starts a new shell in terrain directory or changes directory in shell
  to terrain directory following happens:
  - `all`:
    - a new terrainium shell will be started, if terrain is not active.
    - newly started shell will replace existing shell process.  
       i.e. old shell will be killed and new terrainium shell will remain.
      This works same as using `exec` command in shell.
    - both background and foreground processes will run.
  - `replace`:
    - a new terrainium shell will be started, if terrain is not active.
    - newly started shell will replace existing shell process.  
       i.e. old shell will be killed and new terrainium shell will remain.
      This works same as using `exec` command in shell.
    - background processes will NOT be run, only foreground processes will run
      in newly created shell.
  - `background`:
    - a new terrainium shell will be started, if terrain is not active.
    - newly started shell will be child process of current shell which does not have
      terrainium enabled.
    - both background and foreground processes will run.
    - If user wants to return to shell without terrainium enabled
      they can use this by exiting terrainium shell.
  - `enabled`:
    - a new terrainium shell will be started, if terrain is not active.
    - newly started shell will be child process of current shell which does not have
      terrainium enabled.
    - background processes will NOT be run, only foreground processes will run
      in newly created shell.
    - If user wants to return to shell without terrainium enabled
      they can use this by exiting terrainium shell.
  - `off`:
    - a new terrainium shell will not be started.
    - user will have to manually run `terran enter` to start terrainium shell.

## `default_biome`

- Default biome to be used, when not specified by `-b|--biome` flag.
- When auto apply mechanism is enabled default biome will be used to create terrainium shell.

## `envs`

- list of environment variables and their values.
- if it contains reference to another environment variable `${VAR_NAME}` (`$VAR_NAME` won't work),
  then it will be substituted with environment variable either defined in same terrain
  (including same biome) or system environment variable during runtime.

### `terrain.envs`

- environment variables specified in this section (`terrain.envs`) will be used by all the biomes.
- if same environment variable is specified in a biome, then the value in biome will override value.

### `biomes.<biome_name>.envs`

- environment variables specified in this section (`biome.<biome_name>.envs`) will be used only by this biome.
- the environment variables in this section will override variables in `terrain.envs`.

### Rules for the values specified in this section

1. empty environment variable name (key) is not allowed.
2. if environment variable name (key) start or ends with white spaces (" "),
   those will be automatically removed.
3. if environment variable name contains white space (" ") error will be thrown.
4. if environment variable name starts with number error will be thrown.
5. if environment variable name contains values other than alphabets, numbers, and
   underscore (`[a-zA-Z0-9_]`) error will be thrown.

## `aliases`

- list of aliases and their values.

### `terrain.aliases`

- aliases specified in this section (`terrain.aliases`) will be used by all the biomes.
- if same alias is specified in a biome, then the value in biome will override value.

### `biomes.<biome_name>.aliases`

- aliases specified in this section (`biome.<biome_name>.aliases`) will be used only by this biome.
- the alias in this section will override variables in `terrain.aliases`.

### Rules for the values specified in this section

1. empty alias (key) is not allowed.
2. if alias (key) start or ends with white spaces (" "), those will be automatically removed.
3. if alias contains white space (" ") error will be thrown.
4. if alias starts with number error will be thrown.
5. if alias contains values other than alphabets, numbers, and underscore (`[a-zA-Z0-9_]`)
   error will be thrown.

## `constructors` or `destructors`

### command definition

- `exe`
  - executable file to be executed
  - required
- `args`
  - array of arguments to be provided to executable
  - required
- `cwd`
  - directory in which command is to be executed.
  - optional
  - if not specified command will run in terrain directory (`TERRAIN_DIR` environment variable)
  - if it contains environment variable `${VAR_NAME}` (`$VAR_NAME` won't work),
    then it will be substituted with environment variable either defined terrain.toml or
    system environment variable during runtime.

### `<constructors|destructors>.foreground`

- The commands will run in the shell created by terrainium.
- When `terrain enter`, `terrain construct` is executed, `constructors.foreground`
  will be run in terrainium shell.
- When `terrain exit`, `terrainium destruct` is executed, `destructors.foreground`
  will be run in terrainium shell.

### `<constructors|destructors>.background`

- The commands will be spawned by terrainiumd.
- When `terrain enter`, `terrain construct` is executed, `constructors.background`
  will be spawned.
- When `terrain exit`, `terrainium destruct` is executed, `destructors.background`
  will be spawned.

### `terrain.<constructors|destructors>.<foreground|background>`

- The commands specified in this section will be executed for all biomes.
- The commands specified in `biomes` will be merged with the commands specified
  this section.
- The commands will run in a sequence that they are specified in terrain.toml.
- The commands from `terrain` will run first, and then from biome.

### `biomes.<biome_name>.<constructors|destructors>.<foreground|background>`

- The commands specified in this section will be only when the biome is selected.

### Rules for the values specified in this section

1. `exe` cannot be empty.
2. if `exe` start or ends with white spaces (" "), those will be automatically removed.
3. if `exe` contains white space (" "), error will be thrown.
4. if `exe` for `foreground` command contains `sudo`, a warning will be shown.
5. if `exe` for `background` command contains `sudo`, a warning will be shown and
   error will be thrown when it is spawned by terrainiumd i.e. it won't be executed.
6. if `exe` is not a path, then the `exe` will be checked if it exists in
   directories specified in `$PATH` environment variable.
7. if `exe` exists in `$PATH`, but not executable error will be thrown.
8. if `exe` is path (absolute or relative) to a file, and path does not exists, error will be thrown.
9. if `exe` is path to a file, and user does not have execute permission, error will be thrown.
10. if `exe` is path to a directory, error will be thrown.
11. if `exe` is symlink to a file, same validation as points 6, 7 will be done.
12. if `exe` is symlink to a directory, error will be thrown.
13. if `cwd` is absolute, and does not exist, error will be thrown.
14. if `cwd` is relative, then it will be joined with terrain directory,
    and that path does not exist error will be thrown.
15. if `cwd` is not a directory, an error will be thrown.
16. if `cwd` is a symlink that does not resolves to directory,
    an error will be thrown.
17. if `cwd` environment variable that is not defined,
    in terrain or system environment variables a warning will be shown.

# Testing

## Expected behavior and Scenarios to test manually

- `--update-rc`:
  - appends rc contents to ~/.zshrc
  - throws error if file does not exist

- `--update-rc-path <path>`:
  - appends rc contents to path input
  - throws error if invalid path

- `init`:
  - throws error if terrain already exists
  - creates the central storage and scripts dir if not present
  - creates a terrain.toml file in same directory
  - creates scripts and zwc files for all biomes in central storage in all scenarios
  - `-c` - creates file in at central storage
  - `-x` - creates a file with example terrain.toml
  - `-e` - creates a file and opens editor
  - `-c -e` creates file in central storage and opens editor
  - `-c -x` creates example file in central storage
  - `-x -e` creates example file locally and opens editor
  - `-c -x -e` creates example file in central storage and opens editor

- `edit`:
  - opens editor to edit terrain.toml
  - If `EDITOR` not set uses `vi`
  - throws error if `EDITOR` is not set and `vi` not found
  - recompiles all zsh and zwc after exiting editor

- `generate`:
  - generates zsh script and zwc for all biomes
  - creates scripts dir if does not exists
  - valid zsh script is generated

- `get`
  - without any option returns all
  - `--auto-apply` returns only auto-apply value
    - `enabled`
    - `replaced`
    - `background`
    - `all`
    - `off`
  - `-b` returns all for specific biome and also of main terrain if not defined
  - `--aliases` returns all aliases
  - `-a` returns alias passed in as option to arg
  - `--aliases` and `-a` cannot be used together
  - `--envs` returns all envs
  - `-e` returns env passed in as option to arg
  - `--envs` and `-e` cannot be used together
  - `-c` returns constructors
  - `-d` returns destructors

- `update`:
  - validate values for all options
  - `--auto-apply` updates auto apply
    - `enable`
    - `replace`
    - `background`
    - `all`
    - `off`
  - `-s` updates default biome
  - throws error if `-s` specifies unknown biome
  - no flag other `--auto-apply` can be used with `-s`
  - `-e` updates / adds env
  - `-a` updates / ads aliases
  - for `-e` and `-a` values should be key value pair separated by `<key>=<value>`
  - `-n` creates a new biome
  - `-e` and `-a` with `-n` updates the new biome
  - `-e` and `-a` with `-b` updates the specified biome
  - `-b` updates appropriate biome
  - `-b` cannot be used with `-n`
  - `-k` creates backup file
  - zsh script and zwc is recompiled after execution

- `enter`
  - shell has started with options
  - by default enters default biome if defined otherwise main terrain
  - `-b` if specified enters specific biome and if terrainium is already enabled
    and `-b` passed can be used to change biome
  - background and foreground constructors called
  - background constructors logged at `/tmp/terrainiumd/<terrain_name>/<session-id>/` dir
  - if `auto_apply` is not `off`
    - `enabled` - creates a nested / sub-shell with only foreground constructors
    - `replaced` - replaces existing shell with only foreground constructors
    - `background` - creates a nested / sub-shell with both foreground constructors
      and background constructors. Only works if `terrainiumd` is already running
    - `all` - replaces existing shell with running both foreground constructors
      and background constructors. Only works if `terrainiumd` is already running

- `status`
  - `terrainiumd` is not running:
    - show an error stating `terrainiumd` is not running
  - executed inside directory of terrain
    - `terrain` is **not** active
      - without any args
        - show status of last run terrain
      - with recent arg
        - show status according to argument:
          - if terrain status is present; show
          - if **no** such terrain status; throw an error
    - `terrain` is active
      - without any args
        - terrain does have background constructors / destructors
          - entered using auto-apply
            - background is enabled
              - show status of active terrain
            - background is disabled
              - show message stating terrain is enabled without background
        - terrain does **not** have background constructors / destructors
          - show message stating there are no background constructors / destructors
      - with recent arg
        - show status according to argument:
          - if terrain status is present; show
          - if **no** such terrain status; throw an error
  - executed outside directory of terrain
    - `terrain` is **not** active
      - show an error stating `terrain` is not present for this directory
    - `terrain` is active
      - same as above when executed inside directory of terrain

- `exit`
  - background and foreground destructors called
  - background destructors logged at `/tmp/terrainiumd/<terrain_name>/<session-id>/` dir
  - if auto-apply and background is not enabled the background destructors are not
    run

- `construct`
  - background and foreground constructors called
  - background constructors logged at `/tmp/terrainium-<session-id>` dir

- `destruct`
  - background and foreground destructors called
  - background destructors logged at `/tmp/terrainium-<session-id>` dir

# Testing

## Expected behavior and Scenarios to test manually

- `init`:

  - creates a terrain.toml file in same directory
  - creates script and zwc files for all biomes in central storage in all scenarios
  - `-c` - creates file in at central storage
  - `-x` - creates a file with example terrain.toml
  - `-e` - creates a file and opens editor
  - `-c -e` creates file in central storage and opens editor
  - `-c -x` creates example file in central storage
  - `-f -e` creates example file locally and opens editor
  - `-c -x -e` creates example file in central storage and opens editor

- `edit`:

  - opens editor to edit terrain.toml
  - recompiles all zsh and zwc after exiting editor

- `update`:

  - `-s` updated default biome
  - no other flag can be used with `-s`
  - `-b` updates appropriate biome
  - `-b` cannot be used with `-n`
  - `-e` updates env
  - `-a` updates aliases
  - for `-e` and `-a` values should be key value pair separated by `<key>=<value>`
  - `-n` creates a new biome
  - `-e` and `-a` with `-n` updates the new biome
  - `-k` creates backup file
  - validate values for all options
  - zsh script and zwc is recompiled after execution

- `generate`:

  - generates zsh script and zwc for all biomes
  - valid zsh script is generated

- `get`

  - without any option returns all
  - `-b` returns all for specific biome and also of main terrain if not defined
  - `--alias` returns all aliases
  - `-a` returns alias passed in as option to arg
  - `--alias` and `-a` cannot be used together
  - `--env` returns all envs
  - `-e` returns env passed in as option to arg
  - `--env` and `-e` cannot be used together
  - `-c` returns constructors
  - `-d` returns destructors

- `enter`

  - shell has started with options
  - by default enters default biome if defined otherwise main terrain
  - `-b` if specified enters specific biome and if terrainium is already enabled
    and `-b` passed can be used to change biome
  - background and foreground constructors called
  - background constructors logged at `/tmp/terrainium-<session-id>` dir

- `exit`

  - by default exits entirely
  - background and foreground destructors called
  - background destructors logged at `/tmp/terrainium-<session-id>` dir

- `construct`

  - background and foreground constructors called
  - background constructors logged at `/tmp/terrainium-<session-id>` dir

- `destruct`
  - background and foreground destructors called
  - background destructors logged at `/tmp/terrainium-<session-id>` dir

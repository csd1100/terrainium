# Testing

## Scenarios to test manually

- `init`:

  - creates a terrain.toml file in same directory
  - creates script and zwc files for all biomes in central storage in all scenarios
  - `-c` - creates file in at central storage
  - `-f` - creates a file with example terrain.toml
  - `-e` - creates a file and opens editor
  - `-c -e` creates file in central storage and opens editor
  - `-c -f` creates example file in central storage
  - `-f -e` creates example file locally and opens editor
  - `-c -f -e` creates example file in central storage and opens editor

- `edit`:

  - opens editor to edit terrain.toml
  - recompiles zsh and zwc after exiting editor

- `update`:

  - zsh script and zwc is recompiled after execution
  - `-s` updated default biome
  - no other flag can be used with `-s`
  - `-b` updates appropriate biome
  - `-b` cannot be used with `-n`
  - `-e` updates env
  - `-a` updates aliases
  - `-n` creates a new biome
  - `-e` and `-a` with `-n` updates the new biome
  - `-k` creates backup file
  - validate values for all options

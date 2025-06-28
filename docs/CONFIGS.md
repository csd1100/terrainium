# Configuration files for terrainium

## Configuration for `terrain`: `terrainium.toml`

- Location: `~/.config/terrainium/terrainium.toml`.
- Can be created by command `terrain --create-config`.

### Options

- `$schema`:
  - JSON schema for this file

- `auto_apply`:
  - whether to disable `auto_apply` globally.
  - type - boolean.
  - default - true.

## Configuration for `terrainiumd`: `terrainiumd.toml`

- Location: `~/.config/terrainium/terrainiumd.toml`.
- Can be created by command `terrainiumd --create-config`.

### Options

- `$schema`:
  - JSON schema for this file.

- `history_size`:
  - size of the recent terrains to be stored.
  - type - number.
  - default - 5.

# terrainium

A command-line utility written in Rust for env management

## Command-Line args (Usage)

```sh
terrainium <verb> [OPTIONS]
```

- Verbs:

  - `init [OPTIONS]` - Generates terrain.toml in current directory or
    central location.

    - `-c|--central` - Stores terrain in `$XDG_CONFIG_HOME/terrainium/terrains/[...parent_]$(pwd).toml`.
    - `-f|--full` - Generates terrain with all possible options.
    - `-e|--edit` - Generates terrain and opens file in `EDITOR`.

  - `edit` - edit terrain with editor specified in `EDITOR` env var.

  - `update OPTIONS` - Updates terrain with options

    - `-s|--set-biome <name>` - set default `biome`.
    - `-b|--biome <biome_value>` - biome to update. Updates default if `default`
      is used. Updates terrain if not specified.
    - `-n|--new <new>` creates a new biome with `name`. If `-e`|`-a` are passed with
      this the env and alias will be set for new biome.
    - `-e|--env <VAR_NAME>=<VAR_VALUE>` adds or updates environment variable `VAR_NAME`
      with value `VAR_VALUE`.
    - `-a|--alias <ALIAS_NAME>=<ALIAS_VALUE>` adds or updates alias `ALIAS_NAME`
      with value `ALIAS_VALUE`.
    - `-k|--backup` creates a backup toml before updating the original in same directory.

  - `get [OPTIONS]` - Get the values that will be applied

    - `-a|--all` - returns all values. It is selected by default.
    - `-b|--biome <name>` - name of the biome for which values to be retrieved.
      `default` to use default biome. `none` to remove biome get values from terrain.
    - `-a|--alias [name]` - returns value of aliases defined. If `name` is provided
      will return value of that specific alias.
    - `-e|--env [name]` - returns value of env vars defined. If `name` is provided
      will return value of that specific env vars.
    - `-c|--constructors` - returns value of constructors defined.
    - `-d|--destructors` - returns value of constructors defined.

  - `enter [OPTIONS]` - applies terrain.

    - `-b|--biome <name>` - name of the biome to be applied. `default` to use
      default biome. `none` to remove biome only use terrain without biome.

  - `exit` - exits terrain.

  - `construct [OPTIONS]` - runs commands specified in construct block.

    - `-b|--biome <name>` - name of the biome to be used. Values can be same as `enter`.

  - `deconstruct [OPTIONS]` - runs commands specified in deconstruct block.

    - `-b|--biome <name>` - name of the biome to be used. Values can be same as `enter`.

  - `-h|--help` - shows help.

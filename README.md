# terrainium

A command-line utility written in Rust for env management

## Command-Line args (Usage)

```sh
terrainium <verb> [OPTIONS]
```

- Verbs:
  - `init [OPTIONS]` - Generates terrain.toml in current directory or
    central location.
    - `-c|--central` - Stores terrain in `$XDG_CONFIG_HOME/terrainium/terrains/[parent_]$(pwd).toml`.
    - `-f|--full` - Generates terrain with all possible options.
    - `-e|--edit` - Generates terrain and opens file in `EDITOR`.
  - `edit` - edit terrain with editor specified in `EDITOR` env var.
  - `update OPTIONS` - Updates terrain with options
    - `-b|--biome <name>` - biome to update. Updates default if `default` is used.
      Updates terrain if not specified.
    - `-e|--env <VAR_NAME> <VAR_VALUE>` adds or updates environment variable `VAR_NAME`
      with value `VAR_VALUE`.
    - `-a|--alias <ALIAS_NAME> <ALIAS_VALUE>` adds or updates alias `ALIAS_NAME`
      with value `ALIAS_VALUE`.
    - `-c|--construct <type> <value>` adds constructor of type `<type>` and
      value `<value>`. `-d` Must be specified along with it.
    - `-d|--deconstruct <type> <value>` adds de-constructor of type `<type>` and
      value `<value>`. Needs `-c` along with it.
  - `enter [OPTIONS]` - applies terrain.
    - `-b|--biome <name>` - name of the biome to be applied. `default` to use
      default biome. `none` to remove biome only use terrain without biome.
  - `exit` - exits terrain.
  - `construct` - runs commands specified in construct block.
    - `-b|--biome <name>` - name of the biome to be used. Values can be same as `enter`.
  - `deconstruct` - runs commands specified in deconstruct block.
    - `-b|--biome <name>` - name of the biome to be used. Values can be same as `enter`.
  - `-h|--help` - shows help.

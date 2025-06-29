# Expected behavior and Scenarios

## creates configuration file

### using `--create-config` argument

**User Input:**

```shell
terrain --create-config
```

**Expected Output:**

- configuration file is created at location `~/.config/terrainium/terrainium.toml`

---

### creates configuration file with logging

**User Input:**

```shell
terrain --create-config -l trace
```

**Expected Output:**

- config file is created at location `~/.config/terrainium/terrainium.toml`
- shows logs of config file creation

---

### create configuration fails with other options

**User Input:**

```shell
# ! will fail
terrain --create-config --update-rc
```

```shell
# ! will fail
terrain --create-config init
```

**Expected Output:**

- error showing `--create-config` cannot be used with other options

---

## setup shell integration

### using `--update-rc` argument

**User Input:**

```shell
terrain --update-rc
```

**Expected Output:**

- shell integration script is created at location `~/.config/terrainium/shell_integration/`
- `~/.zshrc` file is updated to source shell integration script

---

### using `--update-rc` with path specified

**User Input:**

```shell
terrain --update-rc ~/zsh/source.zsh
```

**Expected Output:**

- shell integration script is created at location `~/.config/terrainium/shell_integration/`
- `~/zsh/source.zsh` file is updated to source shell integration script

---

### fails when shell other than zsh is used

**User Input:**

```shell
# ! will fail
#  SHELL environment variable does not contain zsh OR is not set
terrain --update-rc
```

**Expected Output:**

- fails with error as other shells are not supported yet

---

## set logging level

### set logging level

**User Input:**

```shell
terrain validate -l trace
```

**Expected Output:**

- prints validation messages from trace level

---

### fails for invalid logging level

**User Input:**

```shell
# ! will fail
terrain validate -l any
```

**Expected Output:**

- fails with error showing invalid logging level

---

## initializes terrain

### using `init` command

**User Input:**

```shell
terrain init
```

**Expected Output:**

- creates `terrain.toml` in current directory.

---

### with example terrain

**User Input:**

```shell
terrain init -x
```

**Expected Output:**

- creates `terrain.toml` in current directory with example terrain included.

---

### inside central directory

**User Input:**

```shell
terrain init -c
```

**Expected Output:**

- creates `terrain.toml` in central directory.
- if current directory is `/home/user/work/project`, then `terrain.toml` file is created in
  `~/.config/terrainium/terrains/_home_user_work_project/`.

---

### launches editor

**User Input:**

```shell
terrain init -e
```

**Expected Output:**

- creates `terrain.toml` in current directory.
- launches editor specified in `EDITOR` environment variable
- if `EDITOR` not specified uses `vi`

---

### combined flags

**User Input:**

```shell
terrain init -cxe
```

**Expected Output:**

- creates `terrain.toml` in central directory.
- launches editor specified in `EDITOR` environment variable
- if `EDITOR` not specified uses `vi`

---

### fails if terrain already exists

**User Input:**

```shell
# ! will fail
terrain init
terrain init
```

```shell
# ! will fail
terrain init -c # creates terrain in central directory
terrain init
```

```shell
# ! will fail
terrain init
terrain init -c # creates terrain in central directory
```

**Expected Output:**

- fails with error notifying user that terrain is already present

---

### generates scripts after `init`

**User Input:**

```shell
terrain init
```

**Expected Output:**

- updates and compiles scripts for new terrain in
  `~/.config/terrainium/terrains/_home_user_work_new_terrain/scripts/`

---

## edits the terrain

### launches `EDITOR` to edit terrain

**User Input:**

```shell
terrain edit
```

**Expected Output:**

- launches text editor specified in EDITOR environment variable

---

### launches `vi` if `EDITOR` not set

**User Input:**

```shell
# unexport EDITOR
typeset +x EDITOR
terrain edit
```

**Expected Output:**

- launches `vi` to edit terrain.toml

---

### generates scripts after `edit`

**User Input:**

```shell
terrain edit
```

**Expected Output:**

- updates and compiles scripts in `~/.config/terrainium/terrains/_home_user_work_terrain/scripts/`

---

## updates terrain

### sets default biome

**User Input:**

```shell
terrain update -s example_biome
```

**Expected Output:**

- updates `default_biome` in `terrain.toml` to `example_biome`

---

### `--set-default` throws error with other arguments, except `--active`, `--backup`

**User Input:**

```shell
# ! will fail
terrain update -s example_biome -b example_biome -e VAR1="SOME VALUE"
```

```shell
# ! will fail
terrain update -s example_biome -n new_biome -e VAR1="SOME VALUE"
```

```shell
# ! will fail
terrain update -s example_biome --auto-apply off
```

**Expected Output:**

- fails with error `--set-default` cannot be used with other variables

---

### updates default biome

**User Input:**

```shell
terrain update  -e VAR1="SOME VALUE" -a greet="echo hello world!"
```

**Expected Output:**

- updates `default_biome`
- adds or updates environment variable `VAR1`
- adds or updates alias `greet`

---

### updates specified biome

**User Input:**

```shell
terrain update -b example_biome -e VAR1="SOME VALUE" -a greet="echo hello world!"
```

**Expected Output:**

- updates `example_biome`
- adds or updates environment variable `VAR1`
- adds or updates alias `greet`

---

### updates multiple environment variables and aliases

**User Input:**

```shell
terrain update -e VAR1="SOME VALUE" -a alias="echo hello world!" \
  -e VAR2="SOME VALUE" -a alias2="echo hello world!"

```

**Expected Output:**

- updates `default_biome`
- adds or updates environment variables `VAR1`, `VAR2`
- adds or updates alias `alias1`, `alias2`

---

### adds new biome

**User Input:**

```shell
terrain update -n new_biome -e VAR1="SOME VALUE" -a greet="echo hello world!"
```

**Expected Output:**

- creates `new_biome`
- adds environment variable `VAR1` in `new_biome`
- adds alias `greet` in `new_biome`

---

### updates auto-apply

**User Input:**

```shell
terrain update --auto-apply off
terrain update --auto-apply enabled
terrain update --auto-apply background
terrain update --auto-apply all
terrain update --auto-apply replace
```

**Expected Output:**

- updates `auto-apply` value

### backs up terrain.toml

**User Input:**

```shell
terrain update --auto-apply replace --backup
```

**Expected Output:**

- updates `auto-apply` value
- creates the backup in `terrain.toml.bkp` file

### updates current terrain

**User Input:**

```shell
cd ~/work/active_terrain
terrain enter
cd ~/work/other_terrain
terrain update -e ENV_VAR="value" --active
```

**Expected Output:**

- updates `active_terrain`
- sets `ENV_VAR` in `active_terrain`'s default biome

---

### `--set-default` throws error if biome does not exist

**User Input:**

```shell
# ! will fail
terrain update -s unknown_biome
```

**Expected Output:**

- fails with error `unknown_biome` does not exist

---

### `--biome` throws error if biome does not exist

**User Input:**

```shell
# ! will fail
terrain update -b unknown_biome -e ENV_VAR=VALUE
```

**Expected Output:**

- fails with error `unknown_biome` does not exist

---

### generates scripts after `update`

**User Input:**

```shell
terrain update -s example_biome
```

**Expected Output:**

- updates and compiles scripts in `~/.config/terrainium/terrains/_home_user_work_terrain/scripts/`

---

## Generates scripts

### `generate`s scripts

**User Input:**

```shell
terrain generate
```

**Expected Output:**

- creates and compiles scripts in `~/.config/terrainium/terrains/_home_user_work_terrain/scripts/`

---

## Validates terrain.toml

### `validate`s scripts

**User Input:**

```shell
terrain validate
```

**Expected Output:**

- validates terrain in current directory.

---

More information about validations performed is in: [TERRAIN.md](./TERRAIN.md)

---

## Fetch values

### `get` in text format

**User Input:**

```shell
terrain get
```

**Expected Output:**

- Fetches values for `default_biome`
- Output similar to [this](../tests/data/terrain-default.rendered).

### `get` the main terrain

**User Input:**

```shell
terrain get -b none
```

**Expected Output:**

- Fetches values for main terrain

---

### `get` using json format

**User Input:**

```shell
terrain get -j
```

**Expected Output:**

- Fetches values for `default_biome` in json format
- Output will be similar to [this](../example_configs/terrain-example_biome.json).

---

### `get` all environment variables and aliases

**User Input:**

```shell
terrain get --aliases --envs
```

**Expected Output:**

- Fetches values for all aliases and environment variables.

---

### `get` specified environment variables and aliases

**User Input:**

```shell
terrain get -a tenter -a non_existent -e EDITOR -e NON_EXISTENT
```

**Expected Output:**

- Fetches specified values
- Output will be:

```
Environment Variables:
    EDITOR="nvim"
    NON_EXISTENT="!!!DOES_NOT_EXIST!!!"
Aliases:
    non_existent="!!!DOES_NOT_EXIST!!!"
    tenter="terrain enter --biome example_biome"
```

---

### `get` fails when both all values and specified value are specified

**User Input:**

```shell
# ! will fail
terrain get --envs -e EDITOR -e NON_EXISTENT
```

```shell
# ! will fail
terrain get --aliases -a tenter
```

**Expected Output:**

- fails specifying both of these cannot be used together

---

### `get` fails when json and other option is specified

**User Input:**

```shell
# ! will fail
terrain get -j --envs
```

**Expected Output:**

- fails specifying both of these cannot be used together

---

## Activation

### `enter`s terrain

**User Input:**

```shell
terrain enter
```

**Expected Output:**

- starts the shell
- sets aliases and environment variables
- runs foreground constructors
- triggers background constructors

---

### `enter`s specified biome

**User Input:**

```shell
terrain enter -b example_biome
```

**Expected Output:**

- uses biome `example_biome`
- starts the shell
- sets aliases and environment variables
- runs foreground constructors
- triggers background constructors

---

## Constructors

### runs constructors

**User Input:**

```shell
terrain construct
```

**Expected Output:**

- runs foreground constructors
- triggers background constructors

---

### `construct` specified biome

**User Input:**

```shell
terrain construct -b example_biome
```

**Expected Output:**

- uses biome `example_biome`
- runs foreground constructors
- triggers background constructors

---

## Destructors

### runs destructors

**User Input:**

```shell
terrain destruct
```

**Expected Output:**

- runs foreground destructors
- triggers background destructors

---

### `destruct` specified biome

**User Input:**

```shell
terrain destruct -b example_biome
```

**Expected Output:**

- uses biome `example_biome`
- runs foreground destructors
- triggers background destructors

---

## Deactivation

### `exit`s terrain

**User Input:**

```shell
terrain exit
```

**Expected Output:**

- runs foreground destructors
- triggers background destructors
- exits shell

---

### `exit` fails if terrain is not active

**User Input:**

```shell
# ! will fail
terrain exit
```

**Expected Output:**

- fails with the error stating terrain is not active

---

## Status

### terrains `status`

**User Input:**

```shell
terrain status -t <terrain_name> -s <session_id>
```

**Expected Output:**

- fetches status for terrain `terrain_name` and session id `session_id`

---

### terrains most recent `status`

**User Input:**

```shell
terrain status -t <terrain_name>
```

```shell
terrain status -t <terrain_name> -r 0
```

**Expected Output:**

- fetches last updated sessions' status for terrain `terrain_name` if terrain is not
  active

---

### terrains recent `status`

**User Input:**

```shell
terrain status -t <terrain_name> -r 2
```

**Expected Output:**

- fetches 2nd most recent updated sessions' status for terrain `terrain_name`

---

### terrains fetches active sessions `status`

**User Input:**

```shell
terrain status
```

**Expected Output:**

- fetches status of active session by reading `TERRAIN_NAME` and `TERRAIN_SESSION_ID`
  environment variable

---

## Terrainium Daemon

### starts daemon socket

**User Input:**

```shell
terrainiumd --run
```

**Expected Output:**

- starts the daemon to handle background commands
- creates a socket at `/tmp/terrainiumd/socket location`
- creates a pid file at `/tmp/terrainiumd/pid`

---

### fails to create socket if already running

**User Input:**

```shell
# ! will fail
terrainiumd --run &> /dev/null &
terrainiumd --run
```

**Expected Output:**

- fails as terrainiumd is already running

---

### kills older process and starts a new one

**User Input:**

```shell
terrainiumd --run &> /dev/null &
terrainiumd --run --force
```

**Expected Output:**

- kills first instance of terrainiumd and starts a new one

---

### creates a configuration

**User Input:**

```shell
terrainiumd --create-config
```

**Expected Output:**

- creates the configuration file at `~/.config/terrainium/terrainiumd.toml`

---

### `install` terrrainiumd service

**User Input:**

```shell
terrainiumd install
```

**Expected Output:**

- installs service file
- enables the service
- starts the service

---

### `install` terrrainiumd twice, shows already installed

**User Input:**

```shell
# will NOT fail but show already installed and reload the service if needed
terrainiumd install
terrainiumd install
```

**Expected Output:**

- reloads the service

---

### `remove`s terrrainiumd service

**User Input:**

```shell
terrainiumd remove
```

**Expected Output:**

- stops the service
- removes the service file
- reloads service manager

---

### `remove` without `install` fails

**User Input:**

```shell
# ! will fail
terrainiumd remove
terrainiumd remove
```

**Expected Output:**

- fails if service is not installed

---

### `enable`s service

**User Input:**

```shell
terrainiumd enable
```

**Expected Output:**

- enables service to be started on logon

---

### `enable`s service `--now`

**User Input:**

```shell
terrainiumd enable --now
```

**Expected Output:**

- enables service to be started on logon
- starts the service if not started

---

### `disable`s service

**User Input:**

```shell
terrainiumd disable
```

**Expected Output:**

- disables service to be started on logon

---

### `disable`s service `--now`

**User Input:**

```shell
terrainiumd disable --now
```

**Expected Output:**

- disables service to be started on logon
- stops the service if running

---

### `start`s service

**User Input:**

```shell
terrainiumd start
```

**Expected Output:**

- starts the service if not running

---

### `start` fails if already running

**User Input:**

```shell
# ! will fail
terrainiumd start
terrainiumd start
```

**Expected Output:**

- fails with error service is already running

---

### `stop`s service

**User Input:**

```shell
terrainiumd stop
```

**Expected Output:**

- stops the service if service is running

---

### `stop` fails if not running

**User Input:**

```shell
# ! will fail
terrainiumd stop
terrainiumd stop
```

**Expected Output:**

- fails with error service is not running

---

### `reload`s service

**User Input:**

```shell
terrainiumd status # Output: "not loaded"
terrainiumd reload
terrainiumd status # Output: "running" / "not running"
```

**Expected Output:**

- Just reloads the service in the system (launchd, systemd).
- Does NOT start the service. On macOS if service is enabled then this will
  start the service as `RunAtLoad` is set to true in launchd service.

---

### `status` shows status

**User Input:**

```shell
terrainiumd status # Output: "not installed"
terrainiumd install
terrainiumd status # Output: "running (enabled)"
terrainiumd stop
terrainiumd status # Output: "not running (enabled)"
terrainiumd disable
terrainiumd status # Output: "not running (disabled)"
terrainiumd remove
terrainiumd status # Output: "not installed"
```

**Expected Output:**

- shows status of the service

---

# Template

## Description

### Use Case

**User Input:**

**Expected Output:**

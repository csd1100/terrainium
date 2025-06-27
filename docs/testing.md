# Testing

Expected behavior and Scenarios to test manually

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

### using `--update-rc-path` argument

**User Input:**

```shell
terrain --update-rc-path ~/zsh/source.zsh
```

**Expected Output:**

- shell integration script is created at location `~/.config/terrainium/shell_integration/`
- `~/zsh/source.zsh` file is updated to source shell integration script

---

### errors when `--update-rc` and `--update-rc-path` is used together

**User Input:**

```shell
# ! will fail
terrain --update-rc --update-rc-path ~/zsh/source.zsh
```

**Expected Output:**

- fails with error both of the options cannot be used together

---

### fails when shell other than zsh is used

**User Input:**

```shell
# ! will fail
#  SHELL env var does not contain zsh OR is not set
terrain --update-rc
```

**Expected Output:**

- fails with error as other shells are not supported yet

---

# Template

## Description

### Use Case

**User Input:**

**Expected Output:**

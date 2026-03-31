# Extensions Guide

## Overview

Pup extensions are standalone executables that add new subcommands to pup. When you run `pup terraform ...`, pup checks if `terraform` is a built-in command. If not, it looks for an installed extension named `pup-terraform` and runs it with your arguments and auth credentials.

Extensions let teams ship experimental features independently without modifying pup's core or doing a full release. Any language works - extensions are just executables.

## Quick Start

### Install an extension from a local file

```bash
# Install from a local binary
pup extension install --local /path/to/pup-my-tool

# Install as a symlink (for development)
pup extension install --local /path/to/pup-my-tool --link
```

### Use it

```bash
# The extension becomes a pup subcommand
pup my-tool --some-flag value
```

### Manage extensions

```bash
# List installed extensions
pup extension list

# List in table format
pup -o table extension list

# Remove an extension
pup extension remove my-tool
```

## Writing an Extension

An extension is any executable named `pup-<name>`. It can be a shell script, a compiled binary, a Python script with a shebang - anything that can run.

### Minimal example (shell script)

```bash
#!/bin/bash
echo "Hello from pup extension!"
echo "Site: $DD_SITE"
echo "Args: $@"
```

Save this as `pup-hello`, make it executable (`chmod +x pup-hello`), and install it:

```bash
pup extension install --local ./pup-hello
pup hello world
# Output:
# Hello from pup extension!
# Site: datadoghq.com
# Args: world
```

### Naming rules

- The executable must be named `pup-<name>` (or `pup-<name>.exe` on Windows)
- `<name>` must be lowercase letters, digits, and hyphens only, starting with a letter
- `<name>` must not conflict with a built-in pup command (e.g., `monitors`, `logs`, `auth`)

Valid: `pup-terraform`, `pup-cost-report`, `pup-lint`
Invalid: `pup-Terraform`, `pup-2fast`, `pup-my_tool`, `pup-monitors`

## Auth Forwarding

Extensions receive pup's auth credentials via environment variables. This means extensions don't need to implement keychain access, token refresh, or config file parsing.

### Environment variables set by pup

| Variable | Set When | Value |
|---|---|---|
| `DD_ACCESS_TOKEN` | OAuth2 auth is active | Current (non-expired) access token |
| `DD_API_KEY` | API key is configured | API key |
| `DD_APP_KEY` | App key is configured | Application key |
| `DD_SITE` | Always | Datadog site (e.g., `datadoghq.com`) |
| `DD_ORG` | Org is specified | Named org session |
| `PUP_OUTPUT` | Always | Output format (`json`, `table`, `yaml`, `csv`, `tsv`) |
| `PUP_AUTO_APPROVE` | `--yes` flag or agent mode | `true` |
| `PUP_READ_ONLY` | Read-only mode | `true` |
| `PUP_AGENT_MODE` | Agent mode | `true` |

Pup refreshes the OAuth2 token if needed before passing it to the extension, so extensions always receive a valid token.

Variables not active in the current session are explicitly removed from the child environment to prevent stale credentials from leaking through the parent shell.

### Example: using auth in a Python extension

```python
#!/usr/bin/env python3
import os, requests

token = os.environ.get("DD_ACCESS_TOKEN")
site = os.environ.get("DD_SITE", "datadoghq.com")

headers = {"Authorization": f"Bearer {token}"} if token else {
    "DD-API-KEY": os.environ.get("DD_API_KEY", ""),
    "DD-APPLICATION-KEY": os.environ.get("DD_APP_KEY", ""),
}

resp = requests.get(f"https://api.{site}/api/v1/dashboard", headers=headers)
print(resp.json())
```

### Example: using auth in a Rust extension

Extensions written in Rust can use the `datadog-api-client` crate. The standard Datadog SDK env vars (`DD_API_KEY`, `DD_APP_KEY`, `DD_SITE`) are forwarded automatically, so most SDKs will work without any extra configuration.

## Global Flags

Pup's global flags (`--output`, `--yes`, `--agent`, `--read-only`, `--org`) are parsed by pup before dispatching to the extension. They are NOT passed as CLI arguments to the extension - instead, they are forwarded as environment variables (see the table above).

```bash
# --output table is consumed by pup, extension receives PUP_OUTPUT=table
pup --output table my-tool do-something

# The extension receives only: ["do-something"]
# Not: ["--output", "table", "do-something"]
```

Extension-specific flags (anything pup doesn't recognize) are passed through to the extension unchanged:

```bash
pup my-tool plan --workspace prod --var-file vars.tfvars
# Extension receives: ["plan", "--workspace", "prod", "--var-file", "vars.tfvars"]
```

## Installation Details

### Local install (copy)

```bash
pup extension install --local /path/to/pup-my-tool
```

Copies the binary into pup's extensions directory and sets executable permissions.

### Local install (symlink)

```bash
pup extension install --local /path/to/pup-my-tool --link
```

Creates a symlink instead of copying. Useful during development so changes to the source binary take effect immediately without reinstalling.

### Custom name

```bash
pup extension install --local /path/to/my-binary --name my-tool
```

By default, the extension name is derived from the filename (stripping `pup-` prefix and `.exe` suffix). Use `--name` to override.

### Force reinstall

```bash
pup extension install --local /path/to/pup-my-tool --force
```

Overwrites an existing extension with the same name.

## Extension Directory

Extensions are stored in pup's config directory:

```
<config_dir>/extensions/
  pup-my-tool/
    pup-my-tool          # the executable
    manifest.json        # metadata (written by pup at install time)
```

The config directory location depends on your platform:
- **macOS**: `~/Library/Application Support/pup/extensions/`
- **Linux**: `~/.config/pup/extensions/` (or `$XDG_CONFIG_HOME/pup/extensions/`)
- **Windows**: `%APPDATA%\pup\extensions\`

Override with `PUP_CONFIG_DIR` environment variable.

## Exit Codes

Pup propagates the extension's exit code. If the extension exits with code 1, pup exits with code 1. On Unix, if the extension is killed by a signal, pup exits with 128 + signal number (standard convention).

## Read-Only Mode

When pup runs in read-only mode (`--read-only`), the built-in `pup extension install`, `pup extension remove`, and `pup extension upgrade` commands are blocked. Extension dispatch itself is not blocked - instead, `PUP_READ_ONLY=true` is forwarded and the extension is responsible for honoring it.

## Migrating a Feature to an Extension

To extract an existing pup feature into an extension:

1. Create a standalone executable that implements the feature
2. Read auth from environment variables instead of calling pup's internal auth
3. Name it `pup-<feature>` and test it via `pup extension install --local`
4. Remove the feature from pup's core `Commands` enum
5. Distribute the extension binary separately

## Limitations

- **GitHub-based installation** (`pup extension install owner/repo`) is not yet implemented. Use `--local` for now.
- **Extension upgrade** (`pup extension upgrade`) is not yet implemented.
- **Source must be a regular file**: `pup extension install --local` requires the source path to be a regular file, not a directory.
- **Agent-mode help**: `pup --agent <ext-name> --help` prints pup's top-level schema, not the extension's help. In normal mode, `--help` is passed through to the extension.

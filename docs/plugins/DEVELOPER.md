# Connection Plugin Developer Guide

This document is for third-party developers, and explains how to write, debug, and distribute connection plugins for JumpServer Client.

## Quick Start

### 1. Copy the Demo

```bash
cp -r plugins/demo/hello-terminal plugins/my-company-my-tool
```

### 2. Edit manifest.json

- Change `id` to a globally unique value (reverse domain notation recommended: `com.yourcompany.toolname`)
- Fill in `category`, `protocols`, `comment`, etc.

### 3. Write connect.json

Configure the `executable` and `launch` strategy for each target platform (see [DESIGN.md](./DESIGN.md)).

### 4. Prepare the icon

- `icon.png`, 128×128 PNG, transparent background preferred

### 5. Local testing

**Method A — Directory install (recommended for development)**

Copy the plugin directory to the user config directory:

```bash
# macOS / Linux
cp -r plugins/demo/hello-terminal \
  ~/Library/Application\ Support/jumpserver-client/plugins/demo.hello-terminal

# Windows (PowerShell)
Copy-Item -Recurse plugins\demo\hello-terminal `
  "$env:APPDATA\jumpserver-client\plugins\demo.hello-terminal"
```

After restarting the client, you should see "Hello Terminal (Demo)" under **Settings → Apps → SSH**.

**Method B — Packaged install**

```bash
./plugins/tools/pack.sh plugins/demo/hello-terminal
# Generates dist/demo.hello-terminal@1.0.0.jscplugin
# Select this file to install it under the client's "Settings → Plugin Management" (a phase 2 feature)
```

### 6. Package and distribute

Distribute the `.jscplugin` file to users, or publish it to an enterprise plugin repository.

---

## manifest.json field reference

```json
{
  "id": "demo.hello-terminal",
  "name": "hello_terminal",
  "display_name": "Hello Terminal (Demo)",
  "version": "1.0.0",
  "min_client_version": "4.0.0",
  "author": "JumpServer Community",
  "homepage": "https://github.com/matheus-marques-ft/js-client",
  "download_url": "",
  "category": "terminal",
  "protocols": ["ssh"],
  "builtin": false,
  "comment": {
    "zh": "演示插件：通过脚本启动终端。",
    "en": "Demo plugin: launches terminal via script."
  }
}
```

**Note**:

- `id` cannot be changed after installation; when upgrading a plugin, bump `version` and keep `id` unchanged
- `category` must match the actual purpose, since it determines which settings sub-page it appears on
- The protocols in `protocols` must already be supported in JumpServer

---

## connect.json in detail

### Simple argument mode (args)

For tools with a fixed command-line argument format, such as PuTTY or the DBeaver CLI:

```json
{
  "platforms": {
    "windows": {
      "executable": {
        "type": "user_path",
        "default": "",
        "required": true
      },
      "launch": {
        "type": "args",
        "template": "-ssh {username}@{host} -P {port} -pw {value}"
      }
    }
  }
}
```

`executable.type` values:

| Value | Description |
|----|------|
| `bundled` | Uses the binary bundled with the client (`default` is a relative path) |
| `system` | A command on the system PATH (`default` is the command name, e.g. `putty.exe`) |
| `user_path` | The user must select the executable path in Settings |

### Script mode (script)

For scenarios like iTerm2 that need AppleScript / PowerShell automation.

`connect.json`:

```json
{
  "platforms": {
    "macos": {
      "executable": { "type": "system", "default": "osascript" },
      "launch": {
        "type": "script",
        "script": "scripts/launch.macos.applescript"
      }
    }
  }
}
```

Script conventions:

- Receives the connection context (a JSON string) via the `JMS_CONNECT_JSON` environment variable
- Exit code `0` means success, non-`0` means failure
- Stderr output shows up in the client logs

Example JSON received by the script:

```json
{
  "name": "web_server",
  "protocol": "ssh",
  "username": "JMS-admin",
  "value": "secret-token",
  "host": "10.0.0.1",
  "port": 22,
  "asset": { "id": "...", "name": "...", "address": "10.0.0.1" }
}
```

### URL Scheme mode (url)

For tools like Navicat that launch via a custom protocol:

```json
{
  "launch": {
    "type": "url",
    "template": "navicat://conn.mysql?Conn.Host={host}&Conn.Port={port}&Conn.Username={username}"
  }
}
```

### Temp file mode (file)

For RDP: writes a `.rdp` file first, then opens it.

```json
{
  "launch": {
    "type": "file",
    "extension": "rdp",
    "open_with": "system"
  }
}
```

The client writes the server-provided `file.content` to a temp file, then opens it per-platform.

---

## Multi-protocol plugins

A single plugin can support multiple protocols (e.g. XShell supports both ssh and telnet). Just list them in `manifest.protocols`.

The user picks a default tool independently for each protocol; the same plugin can be selected as the default for multiple protocols.

---

## Debugging tips

1. **Inspect the merged config**: call the Tauri `get_config` command, and confirm the plugin appears in the corresponding `category` array
2. **Check the awaken logs**: `{config_dir}/jumpserver-client/logs/`
3. **Script debugging**: run the script manually and inject the environment variable:

```bash
export JMS_CONNECT_JSON='{"protocol":"ssh","host":"127.0.0.1","port":22,"username":"test","value":"pass","name":"test"}'
osascript plugins/demo/hello-terminal/scripts/launch.macos.applescript
```

---

## FAQ

### Plugin doesn't show up on the settings page?

- Check whether `category` matches the page (e.g. the SSH page corresponds to `terminal` + `ssh`)
- Check whether `platforms` includes the current operating system
- Check whether `plugins-state.json` has `enabled: false` for it

### Path picker on Windows?

When `executable.type` is `user_path`, the settings page shows a "Select Path" button (consistent with the behavior of existing third-party tools).

### Can it depend on a binary bundled with the client?

Yes. Use `executable.type: "bundled"`, with `default` set to a path relative to the client's `resources/bin/`. This type is recommended only for JumpServer's own official built-in plugins.

---

## Version compatibility

- Bumping the plugin's `version` is enough to trigger a reinstall/overwrite
- Installation is rejected if `min_client_version` is higher than the current client version
- New `launch.type` values are declared via `min_client_version`

---

## Submitting an official built-in plugin

If you'd like your plugin included in the official JumpServer release:

1. Submit a PR under `plugins/builtin/`
2. Set `manifest.builtin: true`
3. Provide test records for each platform
4. Put the icon inside the plugin's own directory — **do not** touch the hardcoded mapping in `settingItems.vue`

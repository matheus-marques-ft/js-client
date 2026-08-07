# Hello Terminal — Connection Plugin Demo

A minimal working example of a JumpServer Client connection plugin, used to verify the plugin directory structure, packaging flow, and script launch conventions.

## File overview

| File | Purpose |
|------|------|
| `manifest.json` | Plugin metadata |
| `connect.json` | Launch method per platform |
| `icon.png` | Settings page icon (128×128) |
| `scripts/` | Platform launch scripts |

## Local install test

```bash
# Run from the repo root
PLUGIN_DIR="$(pwd)/plugins/demo/hello-terminal"

# macOS
DEST=~/Library/Application\ Support/jumpserver-client/plugins/demo.hello-terminal
mkdir -p "$(dirname "$DEST")" && cp -R "$PLUGIN_DIR" "$DEST"

# Linux
DEST=~/.config/jumpserver-client/plugins/demo.hello-terminal
mkdir -p "$(dirname "$DEST")" && cp -R "$PLUGIN_DIR" "$DEST"
```

After restarting the client, enable "Hello Terminal (Demo)" under **Settings → Apps → SSH**.

## Packaging

```bash
./plugins/tools/pack.sh plugins/demo/hello-terminal
```

Output: `dist/demo.hello-terminal@1.0.0.jscplugin`

## Turning it into a real tool

1. Change `launch.type` to `args` and fill in the target client's command-line template; or
2. Keep `script`, and call the real executable from within the script; or
3. Use the `url` type to hook into a URL scheme (e.g. Navicat)

See the [Developer Guide](../../docs/plugins/DEVELOPER.md) for details.

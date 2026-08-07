# JumpServer Client Connection Plugins

Splits the app connection config from `config.json` into standalone plugin packages, for easier maintenance and extension.

## Directory structure

```
plugins/
├── windows/                    # Built-in Windows plugins
│   ├── index.json              # Plugin index for the current platform
│   ├── plugins-state.defaults.json
│   └── windows.*/              # Each plugin's directory
├── macos/                      # Built-in macOS plugins
├── linux/                      # Built-in Linux plugins
├── demo/
│   └── hello-terminal/         # Third-party development example
├── schema/                     # JSON Schema
├── tools/
│   └── split-config.py         # Regenerates platform plugins from config.json
```

## Single plugin structure

```
macos.tigervnc/
├── manifest.json    # Metadata (name, protocol, category, description)
├── connect.json     # Current platform's launch method, default path, enabled state, etc.
└── icon.png         # Settings page icon (optional)
```

## Regenerating built-in plugins

After modifying `go-client/config.json`, restore the full config from a backup, then run:

```bash
python3 plugins/tools/split-config.py
```

## Documentation

- [Architecture Design](../docs/plugins/DESIGN.md)
- [Developer Guide](../docs/plugins/DEVELOPER.md)

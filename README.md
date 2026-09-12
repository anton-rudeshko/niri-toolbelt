# niri-toolbelt

Small utilities for [niri](https://github.com/YaLTeR/niri).

## Utilities

### `focus-workspace-or-cycle`

Focus a workspace by index. If already focused, cycle windows within it instead.

```
focus-workspace-or-cycle <index> [prev]
```

`index` must be an integer from 1 to 255. The only optional argument is `prev`.
Run `focus-workspace-or-cycle --help` (or `-h`) for help without connecting to niri.
Exit status is 0 for success or help, 2 for invalid arguments, and 1 for runtime
errors (for example, when niri is unavailable or the workspace does not exist).

Useful as a single keybind that both switches to a workspace and lets you
cycle through its windows without a separate binding.

Example `config.kdl`:

```kdl
binds {
    Mod+1 { spawn "focus-workspace-or-cycle" "1"; }
    Mod+Shift+1 { spawn "focus-workspace-or-cycle" "1" "prev"; }
}
```

## TODO

- `focus-workspace-or-cycle`: support named workspaces (currently index-only)

## Build

```sh
cargo build --release
```

Or via Nix:

```sh
nix build
```

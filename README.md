# niri-toolbelt

Small utilities for [niri](https://github.com/YaLTeR/niri).

## Utilities

### `focus-workspace-or-cycle`

Focus a workspace by index. If already focused, cycle windows within it instead.

Windows are visited from left to right, and from top to bottom within each
column, wrapping at either end. Floating windows and windows without a
scrolling-layout position are skipped. If no eligible window is focused,
forward cycling selects the first window and `prev` selects the last.
An empty layout or an already focused single window is left alone.

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

### `close-window-unless`

Close the focused window unless its `app_id` matches an exception. This protects
selected applications from accidental closure through a niri keybinding.

```
close-window-unless [--] <app_id>...
close-window-unless dota2
close-window-unless dota2 org.example.Editor
```

Supply at least one exact, case-sensitive `app_id`; patterns and substrings do
not match. Use `--` before IDs starting with `-`. No applications are built in.
Run `close-window-unless --help` (or `-h`) for help without connecting to niri.
Exit status is 0 for success or help, 2 for invalid arguments, and 1 for IPC
errors, which are printed to stderr.

If no window is focused, or its `app_id` matches any exception, nothing happens.
A window without an `app_id` is not excluded. Otherwise, the utility sends
`CloseWindow` with the ID returned by `FocusedWindow`, so a subsequent focus
change cannot redirect the close request to another window.

Replace the close binding in `config.kdl`, for example:

```kdl
binds {
    Mod+Q { spawn "close-window-unless" "dota2"; }
}
```

`dota2` is the verified Dota 2 `app_id`. This only protects this keybinding;
other close bindings, application controls, and exit commands still work.

## Known issues

- `focus-workspace-or-cycle`: multiple monitors are not supported. Workspace
  indices are local to each monitor, but the utility currently looks up an
  index across all monitors.

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

## Install with Nix

Add the flake as an input to your system configuration:

```nix
inputs.niri-toolbelt.url = "github:anton-rudeshko/niri-toolbelt";
inputs.niri-toolbelt.inputs.nixpkgs.follows = "nixpkgs";
```

Then add the package to a Home Manager module that has access to `inputs`:

```nix
home.packages = [
  inputs.niri-toolbelt.packages.${pkgs.stdenv.hostPlatform.system}.default
];
```

After activating the configuration, use the command name in niri bindings as
shown above. The standalone flake uses `nixos-26.05`; `follows` reuses the
consumer's nixpkgs instead. Currently the flake supports `x86_64-linux`.

## Checks

```sh
nix develop --command cargo test --locked
nix develop --command cargo clippy --all-targets --locked -- -D warnings
nix develop --command cargo fmt --all -- --check
```

The tests use a local test socket and do not change the running niri session.

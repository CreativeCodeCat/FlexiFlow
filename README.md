# FlexiFlow

A tiling window manager written in Rust and configured in Lua, for both
X11 and Wayland. Pick "FlexiFlow" (X11) or "FlexiFlow (Wayland)", which
runs on the [river](https://codeberg.org/river/river) compositor, at the
login screen. Nothing needs rebuilding to change the configuration.

## Features

- **13 layouts**: tile, monocle, bottom-stack (vertical and
  horizontal), centered master (fixed and floating), deck, spiral,
  dwindle, gapless grid, grid, horizontal grid and n-row grid. Cycle
  through them from the keyboard, or click the layout symbol in the bar.
- **Gaps and cfact**: per-tag, per-monitor outer and inner gaps, with a
  "smart gaps" mode that drops the outer gaps around a lone window, and
  a per-window `cfact` that resizes one window's share of its column.
- **Lua configuration**: keys, mouse buttons, rules, colors, tags,
  layouts, scratchpads and autostart commands are all plain data.
- **A colorful bar**: every tag has its own accent color, and the menu
  button, layout symbol, power button and window title can each have
  their own. Status text understands inline `^c#RRGGBB^` (foreground),
  `^b#RRGGBB^` (background) and `^d^` (reset) color codes.
- **flexiflow_bar**: a companion status program with its own Lua
  config, showing updates, kernel, battery, RAM, CPU, keyboard layout,
  desktop, network, volume and the clock.
- **Scratchpads**: any number of named scratchpads, each toggled on its
  own, matched by window class and started on first use.
- **Hotkeys popup**: `modkey` + `/` shows every described keybinding
  as key badges under color-coded headings, with type-to-filter search.
- **Restart in place**: reload a rebuilt binary or an edited config
  without logging out; open windows survive.
- Multi-monitor support, a system tray, and window swallowing (a
  terminal is replaced by the GUI app it launched, and comes back when
  the app exits).

## Build and install

```sh
make
sudo make install
```

Building needs a Rust toolchain (see `rust-toolchain.toml`), Lua 5.4,
and the Xlib, Xft, Xinerama and fontconfig development headers.

`make install` installs `FlexiFlow`, `flexiflow_bar`, and an X session
entry so display managers offer "FlexiFlow" as a session. It also
installs the Wayland build (river 0.5 or later) and its "FlexiFlow
(Wayland)" session, so the login screen offers both; see
[src/flexiflow-river/README.md](src/flexiflow-river/README.md). `sudo make
uninstall` removes them.

The repository is a Cargo workspace of three crates under `src/`:
[src/flexiflow-x11/](src/flexiflow-x11) for the X11 window manager,
[src/flexiflow-river/](src/flexiflow-river) for river, and
[src/flexiflow-common/](src/flexiflow-common) for what they share (the layout
algorithms, the Lua config used by the river build, IPC, status text and
drawing). Each keeps its own config: `~/.config/flexiflow/config.lua`
for X11 and `~/.config/flexiflow-river/config.lua` for Wayland.

## Using it

Each session has its own README covering how to run it, its
configuration, and its key and mouse bindings:

- [src/flexiflow-x11/README.md](src/flexiflow-x11/README.md) for X11
- [src/flexiflow-river/README.md](src/flexiflow-river/README.md) for Wayland
- [src/flexiflow-common/README.md](src/flexiflow-common/README.md) for
  the library they share

## License

MIT/X Consortium license; see [LICENSE](LICENSE).

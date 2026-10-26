# flexiflow-common

The library FlexiFlow's two builds share:
[flexiflow-x11](../flexiflow-x11), the X11 window manager, and
[flexiflow-river](../flexiflow-river), the river Wayland build. Nothing
in it talks to X11 or Wayland, so a fix here reaches both sessions.

It isn't installed or built on its own. `make` at the top of the
repository builds it as part of the workspace.

## What's here

- **[layouts](src/layouts.rs)**: the 13 layout algorithms (tile,
  monocle, spiral, dwindle, deck, bstack, bstack_horiz, grid, nrow_grid,
  horiz_grid, gapless_grid, centered_master, centered_floating_master)
  and `Gaps`, dwm's vanitygaps. Each is a pure function from the area,
  the window count, `nmaster`, `mfact`, the gaps and each window's
  `cfact` to one rect per window, borders included. `arrange` picks one
  by its config name; `ARRANGE` lists the names with the layout menu's
  descriptions. The X11 build calls them through a thin harness that
  walks its client list and takes off each border width.
- **[config](src/config.rs)**: the river build's Lua config. The
  built-in [config.lua](src/config.lua) runs first, then the user's
  `~/.config/flexiflow-river/config.lua`, falling back to the X11
  `~/.config/flexiflow/config.lua`. [keysyms](src/config/keysyms.rs) is
  the `XK_*` name table the config's keys use. The X11 build still reads
  its own config with its own loader.
- **[ipc](src/ipc.rs)**: the one-line-per-message protocol on the river
  WM's socket (`status`, `action`, `subscribe`, `pointer`, `focused`),
  and `send`, `ask` and `subscribe` for clients such as the bar.
- **[status](src/status.rs)**: parses the status text's inline codes
  (`^c` `^b` `^d` `^o` `^f` `^a`) into colored runs, boxes and click
  areas, and lays the boxes out.
- **[font](src/font.rs)**: fonts by fontconfig name, such as
  `"Hack Nerd Font:size=10"`, drawn with fontdue, with Xft-style
  fallback for characters the main font lacks.
- **[canvas](src/canvas.rs)**: draws rectangles, text cut short with
  "..." and icons into an ARGB pixel buffer. The river bar and the
  hotkeys popup use it.

## Tests

```sh
cargo test -p flexiflow-common
```

The config tests load a real X11 config
([tests/x11-config.lua](tests/x11-config.lua)) to check it still reads
unchanged.

# FlexiFlow

A tiling X11 window manager, written in Rust, built around dwm's
model: one process, one X connection, raw Xlib calls, and a single
event loop driving both window management and the status bar.
Configuration lives in Lua instead of a header file you recompile.

## Features

- **13 layouts**: tile, monocle, bottom-stack (vertical and
  horizontal), centered master (fixed and floating), deck, spiral,
  dwindle, gapless grid, grid, horizontal grid, and n-row grid.
  Cycle through them with the keyboard or by clicking the layout
  symbol in the bar.
- **Gaps and cfact**: per-tag, per-monitor outer/inner gaps (with a
  "smart gaps" mode that collapses outer gaps to zero for a single
  client), plus a per-client `cfact` to resize one window's share of
  the master/stack column independently of the others.
- **Lua configuration**: keys, mouse buttons, rules, colors, tags,
  layouts, scratchpads, and autostart commands are all plain data in
  `src/config.lua`, with no rebuild required to change them.
- **Per-tag bar colors**: each tag gets its own accent color in the
  bar instead of a single shared highlight color, and is tinted with
  it while the pointer is over it. The layout symbol, power button and
  focused window's title take their own colors from `colors.layout`,
  `colors.power` and `colors.title`.
- **Colorized status bar**: the bar's status text understands inline
  `^c#RRGGBB^` (foreground) / `^b#RRGGBB^` (background) / `^d^`
  (reset) color codes. `FlexiFlow -b "text"` feeds new status text
  to an already-running instance; `flexiflow_bar` (`src/bin/
  flexiflow_bar`) is a second binary that runs it on a timer, with its
  own Lua config, covering updates/kernel/battery/ram/cpu/keyboard/
  desktop/network/volume/clock.
- **Layout switching from outside**: `FlexiFlow -l N` switches the
  running instance's selected monitor to `layouts[N]` (counting from
  0, in `src/config.lua`'s order) - handy for scripts and keybindings,
  and what the layout symbol's right-click menu uses.
- **Scratchpads**: any number of named scratchpad slots, each
  independently toggleable, matched by window class and spawned on
  first use.
- **Restart in place**: reload a rebuilt binary or an edited config
  without logging out - open windows survive.
- Multi-monitor support, a system tray, and window swallowing
  (replacing a terminal with the GUI app it launched, and swapping
  back on exit).

## Build

```sh
make
```

Requires a Rust toolchain (see `rust-toolchain.toml`), Lua 5.4,
Xlib/Xft/Xinerama, and fontconfig development headers.

## Install

```sh
sudo make install
```

Installs the `FlexiFlow` binary, the `flexiflow_bar` status bar
binary, and an X session desktop entry (so display managers can offer
"FlexiFlow" as a session). Run `sudo make uninstall` to remove them
again.

## Running

Either select the "FlexiFlow" session from your display manager, or
add this to your `~/.xinitrc`:

```sh
exec FlexiFlow
```

## Configuration

Defaults are compiled into the binary from `src/config.lua`. To
customize without rebuilding, write your own Lua config to
`$XDG_CONFIG_HOME/flexiflow/config.lua` (or `~/.config/flexiflow/config.lua` if
`XDG_CONFIG_HOME` isn't set) - it's loaded fresh on every startup (and
on every restart, see below).

`flexiflow_bar` has its own, separate config the same way: defaults
compiled in from `src/bin/flexiflow_bar/bar.lua`, overridable at
`~/.config/flexiflow/bar.lua`. It controls the update interval, the
foreground color, the cyclic accent-color palette segments are colored
from, and which segments to show and in what order:

```lua
return {
    interval_ms = 500,
    colors = {
        fg = "#283036",
        accents = { { "#A165E8", "#C9A4FF" }, ... },
    },
    segments = { "updates", "kernel", "battery", "ram", "cpu",
                 "keyboard", "desktop", "network", "volume", "clock" },
}
```

### Key bindings

The modifier key (`modkey`) is Super; `s_mod` is Shift+Super. A few
highlights - see `src/config.lua` for the complete list:

| Keys | Action |
|---|---|
| `modkey` + `j`/`k` | Focus next/previous window |
| `s_mod` + `j`/`k` | Swap with next/previous window |
| `s_mod` + `q` | Close the focused window |
| `s_mod` + `space` | Toggle floating |
| `modkey` + `space` | Cycle to the next layout |
| `modkey+Ctrl` + `,`/`.` | Cycle to the previous/next layout |
| `modkey` + `h`/`l` | Shrink/grow the master area |
| `modkey` + `i`/`u` | Increase/decrease master count |
| `s_mod` + `h`/`l`/`o` | Decrease/increase/reset the focused client's `cfact` |
| `modkey` + `g` | Toggle gaps |
| `modkey` + `1`-`9`,`0` | View tag N |
| `modkey+Ctrl` + `1`-`9`,`0` | Toggle viewing tag N |
| `s_mod` + `1`-`9`,`0` | Move focused window to tag N and follow it there |
| `modkey+Alt` + `1`-`9`,`0` | Move focused window to tag N |
| `s_mod+Ctrl` + `1`-`9`,`0` | Toggle focused window's presence on tag N |
| `modkey` + `[`/`]` or `,`/`.` | Focus previous/next monitor |
| `s_mod` + `,`/`.` | Move focused window to previous/next monitor |
| Alt + `Return`/`b` | Toggle a scratchpad terminal/btop |
| `modkey` + `Return` | Spawn a terminal |
| `modkey` + `b` | Show/hide the bar |
| `modkey` + `w` | Show/hide the bar's tags that have no windows and aren't in view |
| `modkey` + `/` | Show a popup of every keybinding that has a description |
| `s_mod` + `r` | Quit |
| `Alt+Ctrl` + `r` | Restart in place (reloads the binary and config) |

### Mouse bindings

| Click | Action |
|---|---|
| Left/right-click a tag in the bar | View/toggle-view that tag |
| `modkey` + left/right-click a tag in the bar | Move/toggle window to that tag |
| Left-click the layout symbol | Cycle to the next layout |
| Right-click the layout symbol | Open a menu at the pointer to jump to any layout via `FlexiFlow -l` |
| Click the power button (next to the layout symbol) | Open a menu at the pointer of `powermenu` entries: logout, restart, shutdown by default |

A keybinding's optional fifth and sixth arguments, `key(mod, keysym,
func, arg, "Open a terminal", "launcher")`, describe it for the
`modkey` + `/` popup, like awesome's hotkeys popup: each group gets a
heading in the next tag's accent, keys sharing their modifiers and
description share a line, and any key or click closes it. The groups
come in the order `hotkeys_groups` lists them (`flexiflow` first by
default), then any others in the order they're first bound.

Both menus run the `menu` command from `src/config.lua`, which is
[fleximenu](https://github.com/CreativeCodeCat/FlexiMenu) by default; any
dmenu-style menu that takes `-p` and `-l` works. With fleximenu, a
layout's or power entry's `color` (`r`, `g`, `y`, `b`, `p`, `h` or `s`)
boxes its symbol or icon in that color while it's selected.
| Middle-click a window's title | Zoom it into the master area |
| `modkey` + drag a window | Move it |
| `modkey` + middle-click a window | Toggle floating |
| `modkey` + drag with right button | Resize it |

## License

MIT/X Consortium license - see `LICENSE`.

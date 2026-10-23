# flexiflow-rs

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
  bar instead of a single shared highlight color.
- **Colorized status bar**: the bar's status text understands inline
  `^c#RRGGBB^` (foreground) / `^b#RRGGBB^` (background) / `^d^`
  (reset) color codes. `flexiflow-rs -s "text"` feeds new status text
  to an already-running instance; `scripts/flexiflow_bar` is a ready
  example script covering updates/kernel/battery/ram/cpu/keyboard/
  network/volume/clock.
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

Installs the `flexiflow-rs` binary, the `flexiflow_bar` status script,
and an X session desktop entry (so display managers can offer
"flexiflow-rs" as a session). Run `sudo make uninstall` to remove them
again.

## Running

Either select the "flexiflow-rs" session from your display manager, or
add this to your `~/.xinitrc`:

```sh
exec flexiflow-rs
```

## Configuration

Defaults are compiled into the binary from `src/config.lua`. To
customize without rebuilding, write your own Lua config to
`$XDG_CONFIG_HOME/rwm/config.lua` (or `~/.config/rwm/config.lua` if
`XDG_CONFIG_HOME` isn't set) - it's loaded fresh on every startup (and
on every restart, see below).

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
| `s_mod` + `1`-`9`,`0` | Move focused window to tag N |
| `s_mod+Ctrl` + `1`-`9`,`0` | Toggle focused window's presence on tag N |
| `modkey` + `[`/`]` or `,`/`.` | Focus previous/next monitor |
| `s_mod` + `,`/`.` | Move focused window to previous/next monitor |
| Alt + `Return`/`b` | Toggle a scratchpad terminal/btop |
| `modkey` + `Return` | Spawn a terminal |
| `s_mod` + `r` | Quit |
| `s_mod+Ctrl` + `r` | Restart in place (reloads the binary and config) |

### Mouse bindings

| Click | Action |
|---|---|
| Left/right-click a tag in the bar | View/toggle-view that tag |
| `modkey` + left/right-click a tag in the bar | Move/toggle window to that tag |
| Left/right-click the layout symbol | Cycle layout forward/backward |
| Middle-click a window's title | Zoom it into the master area |
| `modkey` + drag a window | Move it |
| `modkey` + middle-click a window | Toggle floating |
| `modkey` + drag with right button | Resize it |

## License

MIT/X Consortium license - see `LICENSE`.

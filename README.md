# FlexiFlow

A tiling window manager for X11, written in Rust and configured in Lua.
It runs as a single process with one event loop that manages windows
and draws the status bar, and nothing needs rebuilding to change its
configuration.

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
entry so display managers offer "FlexiFlow" as a session. `sudo make
uninstall` removes them.

## Running

Pick the "FlexiFlow" session in your display manager, or put this in
`~/.xinitrc`:

```sh
exec FlexiFlow
```

From scripts:

- `FlexiFlow -b "text"` sets the running instance's status text.
- `FlexiFlow -l N` switches the selected monitor to `layouts[N]`,
  counting from 0.

## Configuration

The defaults are built in from [src/config.lua](src/config.lua). To
change them, copy it to `~/.config/flexiflow/config.lua` (or
`$XDG_CONFIG_HOME/flexiflow/config.lua`) and edit it. It's read on
every start and restart.

`flexiflow_bar` works the same way: its defaults come from
[src/bin/flexiflow_bar/bar.lua](src/bin/flexiflow_bar/bar.lua), and
`~/.config/flexiflow/bar.lua` overrides them:

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

### Menus

The layout and power menus, and the `modkey` + `m` launcher, open in
the `menu` command, which is
[fleximenu](https://github.com/CreativeCodeCat/FlexiMenu) by default.
Any dmenu-style menu that takes `-p` and `-l` works. With fleximenu, an
entry's `color` (`r`, `g`, `y`, `b`, `p`, `h` or `s`) boxes its icon in
that color while it's selected.

### Hotkeys popup

`modkey` + `/` lists every keybinding that has a description. Type to
filter it (for example "volume", "gap" or "tag"), Backspace to delete,
and Escape, Return or a click outside it to close it.

A keybinding's optional fifth and sixth arguments describe it and name
its group:

```lua
key(modkey, XK_Return, spawn, { V = termcmd }, "Open a terminal", "launcher")
```

The `hotkeys` table sets the popup's colors and the order, headings
and accent colors of its groups:

```lua
hotkeys = {
    bg = "#1E222A", fg = "#E0E0E0", dim = "#7A8290",
    key_fg = "#ECEFF4", key_bg = "#2E3440",
    columns = 4,
    groups = {
        { name = "launcher", label = "Apps & Launchers", color = "#A6E3A1" },
        { name = "client", label = "Windows", color = "#89B4FA" },
        -- ...
    },
},
```

Groups not listed follow in the order they're first bound, under
their capitalized name.

## Key bindings

`modkey` is Super and `s_mod` is Shift+Super. These are the highlights;
[src/config.lua](src/config.lua) has the full list.

| Keys | Action |
|---|---|
| `modkey` + `j`/`k` | Focus next/previous window |
| `s_mod` + `j`/`k` | Swap with next/previous window |
| `s_mod` + `q` | Close the focused window |
| `s_mod` + `space` | Toggle floating |
| `modkey` + `space` | Next layout |
| `modkey+Ctrl` + `,`/`.` | Previous/next layout |
| `modkey` + `h`/`l` | Shrink/grow the master area |
| `modkey` + `i`/`u` | More/fewer master windows |
| `s_mod` + `h`/`l`/`o` | Decrease/increase/reset the focused window's `cfact` |
| `modkey` + `g` | Toggle gaps |
| `modkey` + `1`-`9`,`0` | View tag N |
| `modkey+Ctrl` + `1`-`9`,`0` | Toggle viewing tag N |
| `s_mod` + `1`-`9`,`0` | Move the focused window to tag N and follow it |
| `modkey+Alt` + `1`-`9`,`0` | Move the focused window to tag N |
| `s_mod+Ctrl` + `1`-`9`,`0` | Toggle the focused window on tag N |
| `modkey` + `[`/`]` or `,`/`.` | Focus previous/next monitor |
| `s_mod` + `,`/`.` | Move the focused window to previous/next monitor |
| `modkey` + `Return` | Open a terminal |
| `modkey` + `m` | Run a program |
| Alt + `Return`/`b` | Toggle the terminal/btop scratchpad |
| `modkey` + `b` | Show/hide the bar |
| `modkey` + `w` | Show/hide empty tags that aren't in view |
| `modkey` + `/` | Show the hotkeys popup (type to filter) |
| `s_mod` + `r` | Quit |
| `Alt+Ctrl` + `r` | Restart in place, reloading the binary and config |

## Mouse bindings

| Click | Action |
|---|---|
| Left-click the menu button (left of the tags) | Run a program |
| Left/right-click a tag | View/toggle viewing that tag |
| `modkey` + left/right-click a tag | Move/toggle the focused window to that tag |
| Left-click the layout symbol | Next layout |
| Right-click the layout symbol | Menu of all layouts |
| Click the power button | Power menu: logout, restart, shutdown |
| Middle-click the window title | Zoom the window into the master area |
| `modkey` + left-drag a window | Move it |
| `modkey` + right-drag a window | Resize it |
| `modkey` + middle-click a window | Toggle floating |

## License

MIT/X Consortium license; see [LICENSE](LICENSE).

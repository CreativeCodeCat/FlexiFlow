# flexiflow-x11

FlexiFlow for X11: the `FlexiFlow` window manager and `flexiflow_bar`,
its status program. It runs as a single process with one event loop
that manages windows and draws the bar.

It's built and installed with the rest of FlexiFlow from the top of the
repository (see the [main README](../../README.md)); building it needs
Lua 5.4 and the Xlib, Xft, Xinerama and fontconfig development headers.
Its layout algorithms come from [flexiflow-common](../flexiflow-common),
shared with the [river build](../flexiflow-river).

## Source

- [src/main.rs](src/main.rs): startup and the command line.
  [src/core.rs](src/core.rs) has the event loop and manages clients,
  monitors and the bar; [src/handlers.rs](src/handlers.rs) handles X
  events.
- [src/key_handlers.rs](src/key_handlers.rs): what keys and buttons can
  run (`view`, `cyclelayout`, `togglescratch`, the hotkeys popup, ...).
- [src/layouts.rs](src/layouts.rs): the harness that applies
  flexiflow-common's layouts to the client list.
- [src/config.rs](src/config.rs) and [src/config.lua](src/config.lua):
  the Lua config loader and the built-in defaults.
- [src/drw.rs](src/drw.rs): Xft drawing for the bar and popups.
- [src/bin/flexiflow_bar/](src/bin/flexiflow_bar): the status program.

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
    bg = "#202326", fg = "#FCFCFC", dim = "#A1A9B1",
    key_fg = "#FCFCFC", key_bg = "#292C30",
    columns = 4,
    groups = {
        { name = "launcher", label = "Apps & Launchers", color = "#27AE60" },
        { name = "client", label = "Windows", color = "#3DAEE9" },
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

## Tests

```sh
cargo test -p FlexiFlow
```

The integration test starts a private `Xvfb :1`, so `:1` must be free.
It runs on the built-in config, not yours.

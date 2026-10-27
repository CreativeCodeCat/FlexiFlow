# flexiflow-river

[FlexiFlow](../..)'s window management, rebuilt for the
[river](https://codeberg.org/river/river) Wayland compositor. River 0.5
leaves every window management decision to a separate client speaking
`river-window-management-v1`; `flexiflow-river` is that client, written
in Rust and configured with the same Lua `config.lua` as the X11
FlexiFlow.

## What's here

- **`flexiflow-river`**, the window manager: dwm's tags (with pertag
  layouts, mfact, nmaster, gaps and bar visibility), all 13 FlexiFlow
  layouts, gaps and cfact, rules, any number of scratchpads, window
  swallowing, floating and fullscreen windows, focus follows mouse,
  `Super`+drag move/resize, borders, and `restart` in place without
  losing any window's tags.
- **`flexiflow-river-bar`**, FlexiFlow's bar as a layer-shell panel on
  every output: the menu button, per-tag accent colors with occupied
  indicators and hover tint, the layout symbol, the power button, the
  focused window's title, and the status text with its `^c` `^b` `^d`
  `^o` `^f` `^a` codes. Clicks run the `buttons` bound to the bar.
- **`flexiflow-river-status`**, `flexiflow_bar` for river: the same
  segments, `bar.lua` and click areas, feeding the bar's status text.

## Build and install

It's built and installed with the X11 FlexiFlow, from the top of the
repository:

```sh
make
sudo make install
```

This needs Rust, Lua 5.4 and fontconfig, and river 0.5 or later
(`pacman -S river`; `river-classic` is the old 0.3 and won't work).
`make install` adds a "FlexiFlow (Wayland)" session for display
managers. To start it from a tty instead:

```sh
flexiflow-river --session
```

That starts river with `flexiflow-river` as its window manager, under
the name FlexiFlow, so fastfetch and other fetch tools report the WM as
FlexiFlow rather than river. (`river -c flexiflow-river` works too, but
they'll call it river.)

## Configuration

`~/.config/flexiflow/config.lua`, the same file the X11 FlexiFlow
reads, is read on every start and restart, so one config drives both;
`wm` is `"river"` here and `"x11"` there, for anything that differs.
The settings that only mean something under X (`resize_hints`, `snap`)
are ignored, and `flexiflow_bar` in `autostart` or `statusclick` runs
`flexiflow-river-status` instead. Without the file, the built-in
[flexiflow-common/src/config.lua](../flexiflow-common/src/config.lua)
is written there.

The settings river adds:

```lua
flexiflow = {
    -- focus whichever window the pointer moves onto
    focusfollowsmouse = true,
    -- the bar program; {} to run another bar
    barcmd = { "flexiflow-river-bar" },
    -- the pointer's xcursor theme (river's default if empty) and size
    cursor = { theme = "", size = 24 },
    ...
}
```

Rules match a window's Wayland `app_id` as `class` and its title as
`title`, by substring like dwm. Wayland has no `instance`, so a rule
that sets one never matches.

To check an edited config before restarting:

```sh
cargo run -p flexiflow-river --example check_config
```

`flexiflow-river-status` reads `~/.config/flexiflow/bar.lua`, the X11
bar's file.

## Scripting

```sh
flexiflow-river -b "status text"         # set the status text
flexiflow-river -l 2                     # switch to layouts[2]
flexiflow-river -c view '{"Ui":4}'       # run any key handler
```

These talk to the running instance over
`$XDG_RUNTIME_DIR/flexiflow-river.$WAYLAND_DISPLAY.sock`; see
[flexiflow-common/src/ipc.rs](../flexiflow-common/src/ipc.rs) for its line protocol.

## Differences from the X11 FlexiFlow

- **The tray is StatusNotifierItem, not XEmbed.** The bar is the
  `org.kde.StatusNotifierWatcher`, so apps that use it (Discord, Steam,
  blueman, `nm-applet --indicator`, ...) show up; XEmbed-only ones like
  pasystray don't, and `snixembed` must not run, or it takes the
  tray's place. Left click activates an item, right click opens its
  menu in `menu`. The `systray*` settings work as before, except
  `systrayonleft`, which puts the tray left of the status text.
- **Bar clicks can't use modifiers**: the bar never has keyboard focus,
  so it can't see them, and `buttons` on the bar with a `mask` are
  skipped. Window bindings (`ClkClientWin`) work as before.
- **No urgency hints**: river's protocol doesn't report them.
- **Menus**: `menu` (fleximenu by default) is an X11 program, so it runs
  under Xwayland; a Wayland-native dmenu (`fuzzel --dmenu`, `wmenu`,
  `bemenu`) works too, as long as it takes `-p` and `-l`.

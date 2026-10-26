-- flexiflow_bar's own config, baked in as the defaults, overridable at
-- $XDG_CONFIG_HOME/flexiflow/bar.lua (or ~/.config/flexiflow/bar.lua).
-- Unlike the window manager's config.lua, this is a single expression:
-- the whole file is just `return { ... }`.
--
-- `colors.accents` is a cyclic palette: segment N (0-indexed, by its
-- position in `segments` below) is colored with
-- `accents[N % #accents]` - {bg, bright_bg} - so with the 10 default
-- segments and 7 accents, colors repeat starting at "network".
return {
    interval_ms = 500,
    -- the font the chips are drawn in, like "Hack Nerd Font:size=9"; ""
    -- for FlexiFlow's own bar font. The bar's height doesn't change, so
    -- a font bigger than the bar's is cut off
    font = "Hack Nerd Font:size=8",
    colors = {
        fg = "#283036",
        -- the border around each chip: a color, "" for the chip's own
        -- darker accent, or "none"
        border = "",
        accents = {
            {
                "#A165E8",
                "#C9A4FF",
            }, -- purple
            {
                "#2BC4C9",
                "#59FFF9",
            }, -- cyan
            {
                "#3DBD3D",
                "#59FF59",
            }, -- green
            {
                "#D4C94A",
                "#FFFF59",
            }, -- yellow
            {
                "#E0813A",
                "#FF9C59",
            }, -- orange
            {
                "#D136C9",
                "#FF59F9",
            }, -- pink
            {
                "#CC3333",
                "#FF5959",
            }, -- red
        },
    },
    -- any of: updates, kernel, battery, ram, cpu, keyboard, desktop,
    -- network, volume, clock - reorder, drop, or repeat as you like.
    -- what clicking a segment's chips runs, through `sh -c`: any of
    -- left, middle, right, scroll_up and scroll_down. Segments listed
    -- here take their clicks; elsewhere on the status text, FlexiFlow's
    -- own ClkStatusText bindings apply
    clicks = {
        updates = {
            left = "${TERMINAL:-alacritty} -e arch-update",
        },
        volume = {
            left = "pavucontrol",
            middle = "pactl set-sink-mute @DEFAULT_SINK@ toggle",
            scroll_up = "pactl set-sink-volume @DEFAULT_SINK@ +5%",
            scroll_down = "pactl set-sink-volume @DEFAULT_SINK@ -5%",
        },
        network = {
            left = "nm-connection-editor",
        },
    },
    segments = {
        "updates",
        "kernel",
        "battery",
        "ram",
        "cpu",
        "keyboard",
        "desktop",
        "network",
        "volume",
        "clock",
    },
}

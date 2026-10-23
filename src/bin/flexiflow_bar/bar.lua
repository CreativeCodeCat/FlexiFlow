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
    colors = {
        fg = "#283036",
        sep_fg = "#434C5E",
        sep_bg = "#283036",
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
    segments = {
        "updates",
        "kernel",
        "battery",
        "ram",
        "cpu",
        "keyboard",
        "network",
        "volume",
        "clock",
    },
}

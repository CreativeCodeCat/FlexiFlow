-- flexiflow-rs config, baked in as this fork's defaults (the same spirit as
-- dwm's config.def.h / flexiflow's config.def.h) rather than left for a
-- user config file to override at runtime.

-- Constructor functions ------------------------------------------------
function key(mod, keysym, func, arg)
    return {
        mod_ = mod,
        keysym = keysym,
        func = func,
        arg = arg,
    }
end

-- builds the {fg, bg, border} triples for `tagcolors`, one per accent in
-- `accents`, pairing each with `fg` as the readable foreground.
function tagcolors(fg, accents)
    local ret = {}
    for i, accent in ipairs(accents) do
        ret[i] = {
            fg,
            accent,
            accent,
        }
    end
    return ret
end

function button(click, mask, button, func, arg)
    return {
        click = click,
        mask = mask,
        button = button,
        func = func,
        arg = arg,
    }
end

-- `focus`, if true, switches the view to the matched client's tag on
-- manage (vanilla dwm never switches view from a rule match).
function rule(
    class,
    instance,
    title,
    tags,
    isfloating,
    isterminal,
    noswallow,
    monitor,
    focus
)
    return {
        class = class,
        instance = instance,
        title = title,
        tags = tags,
        isfloating = isfloating,
        isterminal = isterminal,
        noswallow = noswallow,
        monitor = monitor,
        focus = focus or false,
    }
end

-- One scratchpad slot: `class` is matched against a client's WM_CLASS to
-- recognize it, `cmd` is spawned if it isn't running yet. Each slot gets
-- its own reserved tag bit, so `togglescratch`'s `arg.i` (the slot's
-- index into this list) toggles it independently of the others.
function scratchpad(class, cmd)
    return {
        class = class,
        cmd = cmd,
    }
end

-- For each tag key (N = 1, ..., 9, 0) this inserts:
--
-- * MOD+N           to view the tag
-- * MOD+Ctrl+N       to toggle viewing the tag (dwm's bitmask multi-view)
-- * MOD+Shift+N      to send the focused window to the tag
-- * MOD+Shift+Ctrl+N to toggle the window's presence on the tag
function tagkeys(keys)
    for i, xk in ipairs(
                     {
            XK_1,
            XK_2,
            XK_3,
            XK_4,
            XK_5,
            XK_6,
            XK_7,
            XK_8,
            XK_9,
            XK_0,
        }
                 ) do
        arg = {
            Ui = 1 << (i - 1),
        }
        table.insert(keys, key(modkey, xk, view, arg))
        table.insert(keys, key(modkey | ControlMask, xk, toggleview, arg))
        table.insert(keys, key(s_mod, xk, tag, arg))
        table.insert(keys, key(s_mod | ControlMask, xk, toggletag, arg))
    end
end

-- Theme (flexiflow's accent palette, from the old bar.rs / flexiflow_bar) --

local foreground = "#283036"
local border_normal = "#434C5E"
local border_focused = "#FF59F9"

-- one accent per tag (same accent palette flexiflow_bar's segments use),
-- so each workspace gets its own distinct color in the bar instead of
-- every selected tag sharing border_focused.
local tag_accents = {
    "#A165E8", -- 一 purple
    "#2BC4C9", -- 二 cyan
    "#3DBD3D", -- 三 green
    "#D4C94A", -- 四 yellow
    "#E0813A", -- 五 orange
    "#D136C9", -- 六 pink
    "#CC3333", -- 七 red
    "#3A7BE0", -- 八 blue
    "#2BA98F", -- 九 teal
    "#6655C9", -- 零 indigo
}

modkey = Mod4Mask
altkey = Mod1Mask
s_mod = ShiftMask | modkey

termcmd = {
    "alacritty",
}

menucmd = {
    "menu",
}

-- The three scratchpads from the old main.rs's `NamedScratchPad`s. Each
-- command sets both --title and --class to the same marker string so the
-- title-based match above is unambiguous.
scratchpads = {
    scratchpad(
        "scratchpad_terminal", {
            "alacritty",
            "--title=scratchpad_terminal",
            "--class=scratchpad_terminal,scratchpad_terminal",
        }
    ),
    scratchpad(
        "scratchpad_btop", {
            "alacritty",
            "--class=scratchpad_btop,scratchpad_btop",
            "-e",
            "btop",
        }
    ),
}

-- Spawns, 1:1 with flexiflow's old keys.rs ------------------------------

keys = {
    -- focus / stack
    key(
        modkey, XK_j, focusstack, {
            I = 1,
        }
    ),
    key(
        modkey, XK_k, focusstack, {
            I = -1,
        }
    ),
    key(
        modkey, XK_Tab, focusstack, {
            I = 1,
        }
    ),
    key(
        s_mod, XK_j, pushstack, {
            I = 1,
        }
    ),
    key(
        s_mod, XK_k, pushstack, {
            I = -1,
        }
    ),
    key(
        s_mod, XK_q, killclient, {
            I = 0,
        }
    ),
    key(
        s_mod, XK_space, togglefloating, {
            I = 0,
        }
    ),

    -- layout / mfact / nmaster
    key(
        modkey, XK_space, cyclelayout, {
            I = 1,
        }
    ),
    key(
        modkey | ControlMask, XK_comma, cyclelayout, {
            I = -1,
        }
    ),
    key(
        modkey | ControlMask, XK_period, cyclelayout, {
            I = 1,
        }
    ),
    key(
        modkey, XK_h, setmfact, {
            F = -0.05,
        }
    ),
    key(
        modkey, XK_l, setmfact, {
            F = 0.05,
        }
    ),
    key(
        modkey, XK_i, incnmaster, {
            I = 1,
        }
    ),
    key(
        modkey, XK_u, incnmaster, {
            I = -1,
        }
    ),

    -- cfact (master/stack column share, follows the client)
    key(
        s_mod, XK_h, setcfact, {
            F = -0.25,
        }
    ),
    key(
        s_mod, XK_l, setcfact, {
            F = 0.25,
        }
    ),
    key(
        s_mod, XK_o, setcfact, {
            F = 0.0,
        }
    ),

    -- gaps
    key(
        s_mod, XK_u, incrgaps, {
            I = 1,
        }
    ),
    key(
        modkey | ControlMask | ShiftMask, XK_u, incrgaps, {
            I = -1,
        }
    ),
    key(
        s_mod, XK_i, incrigaps, {
            I = 1,
        }
    ),
    key(
        modkey | ControlMask | ShiftMask, XK_i, incrigaps, {
            I = -1,
        }
    ),
    key(
        modkey | ControlMask, XK_o, incrogaps, {
            I = 1,
        }
    ),
    key(
        modkey | ControlMask | ShiftMask, XK_o, incrogaps, {
            I = -1,
        }
    ),
    key(
        modkey | ControlMask, XK_Up, incrohgaps, {
            I = 1,
        }
    ),
    key(
        modkey | ControlMask | ShiftMask, XK_Up, incrohgaps, {
            I = -1,
        }
    ),
    key(
        modkey | ControlMask, XK_Right, incrovgaps, {
            I = 1,
        }
    ),
    key(
        modkey | ControlMask | ShiftMask, XK_Right, incrovgaps, {
            I = -1,
        }
    ),
    key(
        modkey | ControlMask, XK_Down, incrihgaps, {
            I = 1,
        }
    ),
    key(
        modkey | ControlMask | ShiftMask, XK_Down, incrihgaps, {
            I = -1,
        }
    ),
    key(
        modkey | ControlMask, XK_Left, incrivgaps, {
            I = 1,
        }
    ),
    key(
        modkey | ControlMask | ShiftMask, XK_Left, incrivgaps, {
            I = -1,
        }
    ),
    key(
        modkey | ControlMask | ShiftMask, XK_d, defaultgaps, {
            I = 0,
        }
    ),
    key(
        modkey, XK_g, togglegaps, {
            I = 0,
        }
    ),

    -- screens / monitors
    key(
        modkey, XK_bracketright, focusmon, {
            I = 1,
        }
    ),
    key(
        modkey, XK_bracketleft, focusmon, {
            I = -1,
        }
    ),
    key(
        modkey, XK_comma, focusmon, {
            I = -1,
        }
    ),
    key(
        modkey, XK_period, focusmon, {
            I = 1,
        }
    ),
    key(
        s_mod, XK_comma, tagmon, {
            I = -1,
        }
    ),
    key(
        s_mod, XK_period, tagmon, {
            I = 1,
        }
    ),

    -- scratchpads (indices into the `scratchpads` list below), on Alt
    -- rather than modkey to match the old `{A, -1, ...}` dwm binds
    key(
        altkey, XK_Return, togglescratch, {
            I = 0,
        }
    ),
    key(
        altkey, XK_b, togglescratch, {
            I = 1,
        }
    ),

    -- spawns
    key(
        modkey, XK_Return, spawn, {
            V = termcmd,
        }
    ),
    key(
        s_mod, XK_Return, spawn, {
            V = {
                "nemo",
            },
        }
    ),
    key(
        modkey, XK_m, spawn, {
            V = menucmd,
        }
    ),
    key(
        modkey, XK_p, spawn, {
            V = {
                "passmenu",
            },
        }
    ),
    key(
        modkey, XK_F1, spawn, {
            V = {
                "clipmenu_run",
            },
        }
    ),
    key(
        modkey, XK_F2, spawn, {
            V = {
                "youtube_subs",
            },
        }
    ),
    key(
        s_mod, XK_e, spawn, {
            V = {
                "powermenu_run",
            },
        }
    ),
    key(
        modkey | ControlMask, XK_h, spawn, {
            V = {
                "gnome_boxes",
            },
        }
    ),
    key(
        modkey | ControlMask, XK_b, spawn, {
            V = {
                "google-chrome-stable",
            },
        }
    ),
    key(
        modkey | ControlMask, XK_e, spawn, {
            V = {
                "code",
            },
        }
    ),
    key(
        modkey | ControlMask, XK_d, spawn, {
            V = {
                "discord",
            },
        }
    ),
    key(
        modkey | ControlMask, XK_x, spawn, {
            V = {
                "hexchat",
            },
        }
    ),
    key(
        modkey | ControlMask, XK_i, spawn, {
            V = {
                "inkscape",
            },
        }
    ),
    key(
        modkey | ControlMask, XK_k, spawn, {
            V = {
                "krita",
            },
        }
    ),
    key(
        modkey | ControlMask, XK_p, spawn, {
            V = {
                "pkgbrowser",
            },
        }
    ),
    key(
        modkey | altkey, XK_p, spawn, {
            V = {
                "piper",
            },
        }
    ),
    key(
        s_mod, XK_z, spawn, {
            V = {
                "multimonitorlock-gui",
            },
        }
    ),
    key(
        modkey, XK_Print, spawn, {
            V = {
                "win-shot",
                "-w",
            },
        }
    ),
    key(
        s_mod, XK_Print, spawn, {
            V = {
                "win-shot",
                "-s",
            },
        }
    ),
    key(
        modkey | ControlMask, XK_Print, spawn, {
            V = {
                "win-shot",
                "-f",
            },
        }
    ),
    key(
        modkey | altkey, XK_s, spawn, {
            V = {
                "prime-run",
                "steam",
            },
        }
    ),
    key(
        modkey | altkey, XK_m, spawn, {
            V = {
                "prime-run",
                "minecraft-launcher",
            },
        }
    ),
    key(
        modkey | altkey, XK_r, spawn, {
            V = {
                "prime-run",
                "runescape-launcher",
            },
        }
    ),

    -- XF86 media / brightness keys
    key(
        0, XF86XK_MonBrightnessUp, spawn, {
            V = {
                "brightnessctl",
                "set",
                "5%+",
            },
        }
    ),
    key(
        0, XF86XK_MonBrightnessDown, spawn, {
            V = {
                "brightnessctl",
                "set",
                "5%-",
            },
        }
    ),
    key(
        0, XF86XK_AudioLowerVolume, spawn, {
            V = {
                "amixer",
                "set",
                "Master",
                "5%-",
                "unmute",
            },
        }
    ),
    key(
        0, XF86XK_AudioRaiseVolume, spawn, {
            V = {
                "amixer",
                "set",
                "Master",
                "5%+",
                "unmute",
            },
        }
    ),
    key(
        0, XF86XK_AudioMute, spawn, {
            V = {
                "amixer",
                "set",
                "Master",
                "toggle",
            },
        }
    ),
    key(
        0, XF86XK_AudioPlay, spawn, {
            V = {
                "playerctl",
                "play-pause",
            },
        }
    ),
    key(
        0, XF86XK_AudioPause, spawn, {
            V = {
                "playerctl",
                "play-pause",
            },
        }
    ),
    key(
        0, XF86XK_AudioNext, spawn, {
            V = {
                "playerctl",
                "next",
            },
        }
    ),
    key(
        0, XF86XK_AudioPrev, spawn, {
            V = {
                "playerctl",
                "previous",
            },
        }
    ),

    -- session
    key(
        s_mod, XK_r, quit, {
            I = 0,
        }
    ),
    -- re-exec the running binary in place: picks up a `cargo build`
    -- rebuild and/or a reloaded config.lua without logging out (open
    -- windows survive - see key_handlers.rs::restart)
    key(
        s_mod | ControlMask, XK_r, restart, {
            I = 0,
        }
    ),
}
tagkeys(keys)

rwm = {
    borderpx = 2,
    snap = 32,
    showbar = true,
    topbar = true,
    mfact = 0.5,
    nmaster = 1,
    gaps = 10,
    smartgaps = false,
    -- extra vertical room in the bar beyond the font's own height, so
    -- tags/status text aren't flush against the bar's top/bottom edge
    barpadding = 10,
    resize_hints = false,
    lock_fullscreen = true,
    fonts = {
        "Hack:size=10",
    },
    tags = {
        "一",
        "二",
        "三",
        "四",
        "五",
        "六",
        "七",
        "八",
        "九",
        "零",
    },
    colors = {
        norm = {
            border_normal,
            foreground,
            border_normal,
        },
        sel = {
            foreground,
            border_focused,
            border_focused,
        },
    },
    tagcolors = tagcolors(foreground, tag_accents),
    keys = keys,
    dmenucmd = menucmd,
    rules = {
        -- tag assignment, ported from flexiflow's old rules.rs (all switch
        -- view to the tag on match, like the original `switch_to_tag: true`)
        rule("Alacritty", "", "", 1 << 0, false, false, false, -1, true),
        rule("kitty", "", "", 1 << 0, false, false, false, -1, true),
        rule("Code", "", "", 1 << 1, false, false, false, -1, true),
        rule("nvim", "", "", 1 << 1, false, false, false, -1, true),
        rule("jetbrains-studio", "", "", 1 << 1, false, false, false, -1, true),
        rule("Nemo", "", "", 1 << 2, false, false, false, -1, true),
        rule(
            "Ld-linux-x86-64.so.2", "", "", 1 << 3, false, false, false, -1,
            true
        ),
        rule("Gimp", "", "", 1 << 3, false, false, false, -1, true),
        rule("firefox", "", "", 1 << 5, false, false, false, -1, true),
        rule("Opera", "", "", 1 << 5, false, false, false, -1, true),
        rule("Steam", "", "", 1 << 7, false, false, false, -1, true),
        rule("RuneScape", "", "", 1 << 7, false, false, false, -1, true),
        rule("discord", "", "", 1 << 8, false, false, false, -1, true),
        rule("Hexchat", "", "", 1 << 8, false, false, false, -1, true),
        rule("PkgBrowser", "", "", 1 << 9, false, false, false, -1, true),
        rule("", "", "Event Tester", 1 << 9, false, false, false, -1, true),
        rule("", "", "lazygit", 1 << 9, false, false, false, -1, true),

        -- float-only rules, ported from the old main.rs's `floating_classes`
        -- (tags = 0: stay on whichever tag they open on, just float)
        rule("Multimonitorlock-gui.py", "", "", 0, true, false, false, -1),
        rule("floatmenu", "", "", 0, true, false, false, -1),
        rule("Yad", "", "", 0, true, false, false, -1),
        rule("Peek", "", "", 0, true, false, false, -1),
        rule("Snapper", "", "", 0, true, false, false, -1),
    },
    -- Whether to swallow floating windows
    swallowfloating = false,
    -- If 0, sloppy systray follows monitor, otherwise pin systray to monitor n
    systraypinning = 0,
    systrayonleft = false,
    systrayspacing = 2,
    systraypinningfailfirst = true,
    showsystray = true,
    buttons = {
        button(ClkLtSymbol, 0, Button1, setlayout),
        button(
            ClkLtSymbol, 0, Button3, setlayout, {
                L = 1,
            }
        ),
        button(
            ClkWinTitle, 0, Button2, zoom, {
                I = 0,
            }
        ),
        button(
            ClkStatusText, 0, Button2, spawn, {
                V = termcmd,
            }
        ),
        button(
            ClkClientWin, modkey, Button1, movemouse, {
                I = 0,
            }
        ),
        button(
            ClkClientWin, modkey, Button2, togglefloating, {
                I = 0,
            }
        ),
        button(
            ClkClientWin, modkey, Button3, resizemouse, {
                I = 0,
            }
        ),
        -- the original goal of this migration: click a tag to switch to it
        button(
            ClkTagBar, 0, Button1, view, {
                I = 0,
            }
        ),
        button(
            ClkTagBar, 0, Button3, toggleview, {
                I = 0,
            }
        ),
        button(
            ClkTagBar, modkey, Button1, tag, {
                I = 0,
            }
        ),
        button(
            ClkTagBar, modkey, Button3, toggletag, {
                I = 0,
            }
        ),
    },
    -- M-space/M-C-comma/M-C-period (cyclelayout) visit these in order
    layouts = {
        {
            symbol = "[]=",
            arrange = tile,
        },
        {
            symbol = "[M]",
            arrange = monocle,
        },
        {
            symbol = "[@]",
            arrange = spiral,
        },
        {
            symbol = "[D]",
            arrange = dwindle,
        },
        {
            symbol = "H[]",
            arrange = deck,
        },
        {
            symbol = "TTT",
            arrange = bstack,
        },
        {
            symbol = "===",
            arrange = bstack_horiz,
        },
        {
            symbol = "HHH",
            arrange = grid,
        },
        {
            symbol = "###",
            arrange = nrow_grid,
        },
        {
            symbol = "---",
            arrange = horiz_grid,
        },
        {
            symbol = ":::",
            arrange = gapless_grid,
        },
        {
            symbol = "|M|",
            arrange = centered_master,
        },
        {
            symbol = ">M>",
            arrange = centered_floating_master,
        },
    },
    scratchpads = scratchpads,

    -- Subset of flexiflow's old `autostart[]` worth spawning unconditionally;
    -- anything here that isn't installed just fails silently (same as dwm's
    -- runAutostart, which doesn't check either).
    autostart = {
        {
            "setxkbmap",
            "-option",
            "caps:escape",
        },
        {
            "numlockx",
            "on",
        },
        {
            "dunst",
        },
        {
            "clipmenu-daemon",
        },
        {
            "nm-applet",
        },
        {
            "blueman-applet",
        },
        {
            "pasystray",
        },
        {
            "cbatticon",
        },
        -- status2d bar segments (scripts/flexiflow_bar); needs `make install`
        -- to be on $PATH, same as every other entry here
        {
            "flexiflow_bar",
        },
    },
}

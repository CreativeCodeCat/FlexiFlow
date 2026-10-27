-- flexiflow-river config: FlexiFlow's config.lua, for river. These are
-- the built-in defaults; ~/.config/flexiflow-river/config.lua (or, if
-- that doesn't exist, the X11 FlexiFlow's ~/.config/flexiflow/config.lua)
-- runs after this file and can change any of it.
--
-- `wm` is "x11" under FlexiFlow and "river" under flexiflow-river, so one
-- config.lua can drive both: `if wm == "river" then ... end`.
-- Constructor functions ------------------------------------------------
-- `desc`, if given, is what the binding does: `hotkeys` lists every
-- binding that has one, under the heading `group`
function key(mod, keysym, func, arg, desc, group)
    return {
        mod_ = mod,
        keysym = keysym,
        func = func,
        arg = arg,
        desc = desc,
        group = group,
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
-- * MOD+N            to view the tag
-- * MOD+Ctrl+N       to toggle viewing the tag (dwm's bitmask multi-view)
-- * MOD+Shift+N      to send the focused window to the tag and follow it
-- * MOD+Alt+N        to send the focused window to the tag
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
        table.insert(keys, key(modkey, xk, view, arg, "View tag", "tag"))
        table.insert(
            keys, key(
                modkey | ControlMask, xk, toggleview, arg, "Also show tag",
                "tag"
            )
        )
        table.insert(
            keys, key(
                s_mod, xk, tagview, arg, "Move window to tag and follow", "tag"
            )
        )
        table.insert(
            keys,
            key(modkey | altkey, xk, tag, arg, "Move window to tag", "tag")
        )
        table.insert(
            keys, key(
                s_mod | ControlMask, xk, toggletag, arg,
                "Also put window on tag", "tag"
            )
        )
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
        }, "Focus next window", "client"
    ),
    key(
        modkey, XK_k, focusstack, {
            I = -1,
        }, "Focus previous window", "client"
    ),
    key(
        modkey, XK_Tab, focusstack, {
            I = 1,
        }, "Focus next window", "client"
    ),
    key(
        s_mod, XK_j, pushstack, {
            I = 1,
        }, "Move window down the stack", "client"
    ),
    key(
        s_mod, XK_k, pushstack, {
            I = -1,
        }, "Move window up the stack", "client"
    ),
    key(
        s_mod, XK_q, killclient, {
            I = 0,
        }, "Close window", "client"
    ),
    key(
        s_mod, XK_space, togglefloating, {
            I = 0,
        }, "Toggle floating", "client"
    ),

    -- layout / mfact / nmaster
    key(
        modkey, XK_space, cyclelayout, {
            I = 1,
        }, "Next layout", "layout"
    ),
    key(
        modkey | ControlMask, XK_comma, cyclelayout, {
            I = -1,
        }, "Previous layout", "layout"
    ),
    key(
        modkey | ControlMask, XK_period, cyclelayout, {
            I = 1,
        }, "Next layout", "layout"
    ),
    key(
        modkey, XK_h, setmfact, {
            F = -0.05,
        }, "Shrink master area", "layout"
    ),
    key(
        modkey, XK_l, setmfact, {
            F = 0.05,
        }, "Grow master area", "layout"
    ),
    key(
        modkey, XK_i, incnmaster, {
            I = 1,
        }, "More windows in master", "layout"
    ),
    key(
        modkey, XK_u, incnmaster, {
            I = -1,
        }, "Fewer windows in master", "layout"
    ),

    -- cfact (master/stack column share, follows the client)
    key(
        s_mod, XK_h, setcfact, {
            F = -0.25,
        }, "Shrink window's share", "layout"
    ),
    key(
        s_mod, XK_l, setcfact, {
            F = 0.25,
        }, "Grow window's share", "layout"
    ),
    key(
        s_mod, XK_o, setcfact, {
            F = 0.0,
        }, "Reset window's share", "layout"
    ),

    -- gaps
    key(
        s_mod, XK_u, incrgaps, {
            I = 1,
        }, "Grow all gaps", "gaps"
    ),
    key(
        modkey | ControlMask | ShiftMask, XK_u, incrgaps, {
            I = -1,
        }, "Shrink all gaps", "gaps"
    ),
    key(
        s_mod, XK_i, incrigaps, {
            I = 1,
        }, "Grow inner gaps", "gaps"
    ),
    key(
        modkey | ControlMask | ShiftMask, XK_i, incrigaps, {
            I = -1,
        }, "Shrink inner gaps", "gaps"
    ),
    key(
        modkey | ControlMask, XK_o, incrogaps, {
            I = 1,
        }, "Grow outer gaps", "gaps"
    ),
    key(
        modkey | ControlMask | ShiftMask, XK_o, incrogaps, {
            I = -1,
        }, "Shrink outer gaps", "gaps"
    ),
    key(
        modkey | ControlMask, XK_Up, incrohgaps, {
            I = 1,
        }, "Grow outer horizontal gaps", "gaps"
    ),
    key(
        modkey | ControlMask | ShiftMask, XK_Up, incrohgaps, {
            I = -1,
        }, "Shrink outer horizontal gaps", "gaps"
    ),
    key(
        modkey | ControlMask, XK_Right, incrovgaps, {
            I = 1,
        }, "Grow outer vertical gaps", "gaps"
    ),
    key(
        modkey | ControlMask | ShiftMask, XK_Right, incrovgaps, {
            I = -1,
        }, "Shrink outer vertical gaps", "gaps"
    ),
    key(
        modkey | ControlMask, XK_Down, incrihgaps, {
            I = 1,
        }, "Grow inner horizontal gaps", "gaps"
    ),
    key(
        modkey | ControlMask | ShiftMask, XK_Down, incrihgaps, {
            I = -1,
        }, "Shrink inner horizontal gaps", "gaps"
    ),
    key(
        modkey | ControlMask, XK_Left, incrivgaps, {
            I = 1,
        }, "Grow inner vertical gaps", "gaps"
    ),
    key(
        modkey | ControlMask | ShiftMask, XK_Left, incrivgaps, {
            I = -1,
        }, "Shrink inner vertical gaps", "gaps"
    ),
    key(
        modkey | ControlMask | ShiftMask, XK_d, defaultgaps, {
            I = 0,
        }, "Reset gaps", "gaps"
    ),
    key(
        modkey, XK_g, togglegaps, {
            I = 0,
        }, "Toggle gaps", "gaps"
    ),

    -- screens / monitors
    key(
        modkey, XK_bracketright, focusmon, {
            I = 1,
        }, "Focus next monitor", "screen"
    ),
    key(
        modkey, XK_bracketleft, focusmon, {
            I = -1,
        }, "Focus previous monitor", "screen"
    ),
    key(
        modkey, XK_comma, focusmon, {
            I = -1,
        }, "Focus previous monitor", "screen"
    ),
    key(
        modkey, XK_period, focusmon, {
            I = 1,
        }, "Focus next monitor", "screen"
    ),
    key(
        s_mod, XK_comma, tagmon, {
            I = -1,
        }, "Send window to previous monitor", "screen"
    ),
    key(
        s_mod, XK_period, tagmon, {
            I = 1,
        }, "Send window to next monitor", "screen"
    ),

    -- scratchpads (indices into the `scratchpads` list below), on Alt
    -- rather than modkey to match the old `{A, -1, ...}` dwm binds
    key(
        altkey, XK_Return, togglescratch, {
            I = 0,
        }, "Toggle scratchpad terminal", "scratchpad"
    ),
    key(
        altkey, XK_b, togglescratch, {
            I = 1,
        }, "Toggle scratchpad btop", "scratchpad"
    ),

    -- spawns
    key(
        modkey, XK_Return, spawn, {
            V = termcmd,
        }, "Open a terminal", "launcher"
    ),
    key(
        s_mod, XK_Return, spawn, {
            V = {
                "nemo",
            },
        }, "Nemo", "launcher"
    ),
    key(
        modkey, XK_m, spawn, {
            V = menucmd,
        }, "Run a program", "launcher"
    ),
    key(
        modkey, XK_p, spawn, {
            V = {
                "passmenu",
            },
        }, "Passwords", "launcher"
    ),
    key(
        modkey, XK_F1, spawn, {
            V = {
                "clipmenu_run",
            },
        }, "Clipboard history", "launcher"
    ),
    key(
        modkey, XK_F2, spawn, {
            V = {
                "youtube_subs",
            },
        }, "YouTube subscriptions", "launcher"
    ),
    key(
        s_mod, XK_e, spawn, {
            V = {
                "powermenu_run",
            },
        }, "Power menu", "launcher"
    ),
    key(
        modkey | ControlMask, XK_h, spawn, {
            V = {
                "gnome_boxes",
            },
        }, "GNOME Boxes", "launcher"
    ),
    key(
        modkey | ControlMask, XK_b, spawn, {
            V = {
                "google-chrome-stable",
            },
        }, "Google Chrome", "launcher"
    ),
    key(
        modkey | ControlMask, XK_e, spawn, {
            V = {
                "code",
            },
        }, "VS Code", "launcher"
    ),
    key(
        modkey | ControlMask, XK_d, spawn, {
            V = {
                "discord",
            },
        }, "Discord", "launcher"
    ),
    key(
        modkey | ControlMask, XK_x, spawn, {
            V = {
                "hexchat",
            },
        }, "HexChat", "launcher"
    ),
    key(
        modkey | ControlMask, XK_i, spawn, {
            V = {
                "inkscape",
            },
        }, "Inkscape", "launcher"
    ),
    key(
        modkey | ControlMask, XK_k, spawn, {
            V = {
                "krita",
            },
        }, "Krita", "launcher"
    ),
    key(
        modkey | ControlMask, XK_p, spawn, {
            V = {
                "pkgbrowser",
            },
        }, "Package browser", "launcher"
    ),
    key(
        modkey | altkey, XK_p, spawn, {
            V = {
                "piper",
            },
        }, "Piper", "launcher"
    ),
    key(
        s_mod, XK_z, spawn, {
            V = {
                "multimonitorlock-gui",
            },
        }, "Monitor lock", "launcher"
    ),
    key(
        modkey, XK_Print, spawn, {
            V = {
                "win-shot",
                "-w",
            },
        }, "Screenshot window", "screenshot"
    ),
    key(
        s_mod, XK_Print, spawn, {
            V = {
                "win-shot",
                "-s",
            },
        }, "Screenshot region", "screenshot"
    ),
    key(
        modkey | ControlMask, XK_Print, spawn, {
            V = {
                "win-shot",
                "-f",
            },
        }, "Screenshot screen", "screenshot"
    ),
    key(
        modkey | altkey, XK_s, spawn, {
            V = {
                "prime-run",
                "steam",
            },
        }, "Steam", "launcher"
    ),
    key(
        modkey | altkey, XK_m, spawn, {
            V = {
                "prime-run",
                "minecraft-launcher",
            },
        }, "Minecraft", "launcher"
    ),
    key(
        modkey | altkey, XK_r, spawn, {
            V = {
                "prime-run",
                "runescape-launcher",
            },
        }, "RuneScape", "launcher"
    ),

    -- XF86 media / brightness keys
    key(
        0, XF86XK_MonBrightnessUp, spawn, {
            V = {
                "brightnessctl",
                "set",
                "5%+",
            },
        }, "Brightness up", "media"
    ),
    key(
        0, XF86XK_MonBrightnessDown, spawn, {
            V = {
                "brightnessctl",
                "set",
                "5%-",
            },
        }, "Brightness down", "media"
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
        }, "Volume down", "media"
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
        }, "Volume up", "media"
    ),
    key(
        0, XF86XK_AudioMute, spawn, {
            V = {
                "amixer",
                "set",
                "Master",
                "toggle",
            },
        }, "Mute", "media"
    ),
    key(
        0, XF86XK_AudioPlay, spawn, {
            V = {
                "playerctl",
                "play-pause",
            },
        }, "Play/pause", "media"
    ),
    key(
        0, XF86XK_AudioPause, spawn, {
            V = {
                "playerctl",
                "play-pause",
            },
        }, "Play/pause", "media"
    ),
    key(
        0, XF86XK_AudioNext, spawn, {
            V = {
                "playerctl",
                "next",
            },
        }, "Next track", "media"
    ),
    key(
        0, XF86XK_AudioPrev, spawn, {
            V = {
                "playerctl",
                "previous",
            },
        }, "Previous track", "media"
    ),

    -- the bar, and its tags with no windows
    key(
        modkey, XK_b, togglebar, {
            I = 0,
        }, "Toggle the bar", "bar"
    ),
    key(
        modkey, XK_w, togglevacant, {
            I = 0,
        }, "Show/hide unused tags", "bar"
    ),

    -- every keybinding that has a description, listed in `menu`
    key(
        modkey, XK_slash, hotkeys, {
            I = 0,
        }, "Show keybindings", "flexiflow"
    ),

    -- session
    key(
        s_mod, XK_r, quit, {
            I = 0,
        }, "Quit river", "flexiflow"
    ),
    -- re-exec the running window manager in place: picks up a rebuilt
    -- binary and/or an edited config.lua without logging out (open
    -- windows keep their tags)
    key(
        altkey | ControlMask, XK_r, restart, {
            I = 0,
        }, "Restart flexiflow-river", "flexiflow"
    ),
}
tagkeys(keys)

flexiflow = {
    borderpx = 2,
    showbar = true,
    topbar = true,
    mfact = 0.5,
    nmaster = 1,
    gaps = 10,
    smartgaps = false,
    -- extra vertical room in the bar beyond the font's own height, so
    -- tags/status text aren't flush against the bar's top/bottom edge
    barpadding = 10,
    lock_fullscreen = true,
    -- focus whichever window the pointer moves onto
    focusfollowsmouse = true,
    -- the bar, a layer-shell panel; {} for none (to run another bar)
    barcmd = {
        "flexiflow-river-bar",
    },
    -- the pointer's xcursor theme (river's default if empty) and size
    cursor = {
        theme = "",
        size = 24,
    },
    fonts = {
        "Hack Nerd Font:size=10",
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
        -- the bar's layout symbol and power button (both optional,
        -- drawn in `norm` without them)
        layout = {
            foreground,
            "#2BC4C9",
            "#2BC4C9",
        },
        power = {
            foreground,
            "#CC3333",
            "#CC3333",
        },
        -- the menu button left of the tags, in grey so it never clashes
        -- with the tag accents (drawn in `norm` without this)
        menu = {
            "#FCFCFC",
            "#434C5E",
            "#434C5E",
        },
        -- the focused window's title: on the bar background, in the
        -- focus color (drawn in `sel` without this)
        title = {
            border_focused,
            foreground,
            foreground,
        },
    },
    tagcolors = tagcolors(foreground, tag_accents),
    keys = keys,
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
        rule("Google-chrome", "", "", 1 << 5, false, false, false, -1, true),
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
    buttons = {
        -- left click cycles forward through all of `layouts` below,
        -- matching the M-space key (instead of setlayout's upstream
        -- "toggle last two" behavior); right click opens `layouts` in
        -- `menu` (below) to jump to any of them - a silent no-op if the
        -- menu program isn't installed
        button(
            ClkLtSymbol, 0, Button1, cyclelayout, {
                I = 1,
            }
        ),
        button(ClkLtSymbol, 0, Button3, layoutmenu),
        -- the power button next to the layout symbol opens `powermenu`
        -- (below) in `menu`
        button(ClkPowerButton, 0, Button1, powermenu),
        -- the menu button left of the tags runs `menucmd`, like MOD+m
        button(
            ClkMenuButton, 0, Button1, spawn, {
                V = menucmd,
            }
        ),
        button(
            ClkWinTitle, 0, Button2, zoom, {
                I = 0,
            }
        ),
        button(
            ClkStatusText, 0, Button1, spawn, {
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
    -- M-space/M-C-comma/M-C-period (cyclelayout) visit these in order.
    -- `color` (r g y b p h s) boxes a layout's symbol in that color
    -- while it's selected in the layout menu
    layouts = {
        {
            symbol = "[]=",
            color = "b",
            arrange = tile,
        },
        {
            symbol = "[M]",
            color = "p",
            arrange = monocle,
        },
        {
            symbol = "[@]",
            color = "y",
            arrange = spiral,
        },
        {
            symbol = "[D]",
            color = "y",
            arrange = dwindle,
        },
        {
            symbol = "H[]",
            color = "p",
            arrange = deck,
        },
        {
            symbol = "TTT",
            color = "g",
            arrange = bstack,
        },
        {
            symbol = "===",
            color = "g",
            arrange = bstack_horiz,
        },
        {
            symbol = "HHH",
            color = "r",
            arrange = grid,
        },
        {
            symbol = "###",
            color = "r",
            arrange = nrow_grid,
        },
        {
            symbol = "---",
            color = "r",
            arrange = horiz_grid,
        },
        {
            symbol = ":::",
            color = "r",
            arrange = gapless_grid,
        },
        {
            symbol = "|M|",
            color = "b",
            arrange = centered_master,
        },
        {
            symbol = ">M>",
            color = "b",
            arrange = centered_floating_master,
        },
        -- no arrange function: vanilla dwm's "><>" floating layout
        {
            symbol = "><>",
            color = "h",
        },
    },
    scratchpads = scratchpads,

    -- the bar's menu button, drawn left of the tags (leave it empty for
    -- no button); what clicking it does is bound with ClkMenuButton
    menubutton = "\u{F003B}",

    -- the bar's power button (drawn right after the layout symbol; leave
    -- `symbol` empty for no button) and the menu it opens: picking an
    -- item runs its `command` through `sh -c`. `color` (r g y b p h s)
    -- boxes the label's icon in that color while it's selected
    powermenu = {
        symbol = "\u{F011}",
        items = {
            {
                label = "\u{F08B} Logout",
                command = "loginctl terminate-session \"$XDG_SESSION_ID\"",
                color = "y",
            },
            {
                label = "\u{F021} Restart",
                command = "systemctl reboot",
                color = "b",
            },
            {
                label = "\u{F011} Shutdown",
                command = "systemctl poweroff",
                color = "r",
            },
        },
    },

    -- the keybindings popup (MOD+/): its colors, and its groups in the
    -- order they're shown, each with a heading and an accent of its own.
    -- Groups not listed follow in the order they're first bound, with
    -- an accent from the same palette. The colors are KDE's Breeze Dark
    hotkeys = {
        bg = "#202326",
        fg = "#FCFCFC",
        dim = "#A1A9B1",
        key_fg = "#FCFCFC",
        key_bg = "#292C30",
        -- fewer if the popup would be wider than the screen, more if a
        -- column would be taller than it
        columns = 4,
        groups = {
            {
                name = "launcher",
                label = "Apps & Launchers",
                color = "#27AE60",
            },
            {
                name = "client",
                label = "Windows",
                color = "#3DAEE9",
            },
            {
                name = "flexiflow",
                label = "System",
                color = "#9B59B6",
            },
            {
                name = "layout",
                label = "Layouts",
                color = "#1ABC9C",
            },
            {
                name = "tag",
                label = "Tags",
                color = "#FDBC4B",
            },
            {
                name = "screen",
                label = "Monitors",
                color = "#2980B9",
            },
            {
                name = "gaps",
                label = "Gaps",
                color = "#C9CE3B",
            },
            {
                name = "scratchpad",
                label = "Scratchpads",
                color = "#E93A9A",
            },
            {
                name = "screenshot",
                label = "Screenshots",
                color = "#DA4453",
            },
            {
                name = "bar",
                label = "Bar",
                color = "#A1A9B1",
            },
            {
                name = "media",
                label = "Media",
                color = "#F67400",
            },
        },
    },

    -- the menu program the layout and power menus open (dmenu-style: one
    -- item per line in, the picked line out); FlexiFlow adds `-p <prompt>`
    -- and `-l <number of items>`. These are fleximenu's flags: -M opens
    -- it at the mouse pointer, -z 222 makes it 222 pixels wide, and -h -1
    -- makes each item 2.5 times the font height
    menu = {
        "fleximenu",
        "--fn",
        "FiraCode Nerd Font:size=8",
        "-h",
        "-1",
        "-M",
        "-z",
        "222",
    },

    -- spawned once at startup; anything here that isn't installed just
    -- fails silently. The bar itself is `barcmd`, above.
    autostart = {
        {
            "dunst",
        },
        -- the status text's segments (src/bin/flexiflow-river-status)
        {
            "flexiflow-river-status",
        },
    },
}

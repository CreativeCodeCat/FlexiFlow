/* See LICENSE file for copyright and license details. */

#include <X11/XF86keysym.h>

#define GAPS 10

/* appearance */
static const unsigned int borderpx              = 2;        /* border pixel of windows */
static const unsigned int default_border        = 2;        /* to switch back to default border after dynamic border resizing via keybinds */
static const unsigned int snap                  = 15;       /* snap pixel */
static const unsigned int gappih                = GAPS;     /* horiz inner gap between windows */
static const unsigned int gappiv                = GAPS;     /* vert inner gap between windows */
static const unsigned int gappoh                = GAPS;     /* horiz outer gap between windows and screen edge */
static const unsigned int gappov                = GAPS;     /* vert outer gap between windows and screen edge */
static const int smartgaps                      = 0;        /* 1 means no outer gap when there is only one window */
static const unsigned int systraypinning        = 2;        /* 0: sloppy systray follows selected monitor, >0: pin systray to monitor X */
static const unsigned int systrayspacing        = 4;        /* systray spacing */
static const int systraypinningfailfirst        = 1;        /* 1: if pinning fails,display systray on the 1st monitor,False: display systray on last monitor*/
static int showsystray                          = 1;        /* 0 means no systray */
static const int viewontag         		          = 1;     	  /* 0 means don't view on tag switch */
static const int showbar                        = 1;        /* 0 means no bar */
static const int usedbar                        = 1;        /* 0 means don't use flexiflow_bar script */
static const int showtab                        = showtab_auto;
static const int toptab                         = 0;        /* 0 means bottom tab */
static const int topbar                         = 1;        /* 0 means bottom bar */
static const int user_bh                        = 5;        /* 2 is the default spacing around the bar's font */
static const char buttonbar[]                   = "󰍜";
static const int horizpadbar                    = 5;
static const int vertpadbar                     = 5;
static const int vertpadtab                     = 20;
static const int horizpadtabi                   = 20;
static const int horizpadtabo                   = 20;
static const int colorfultag                    = 1;        /* 0 means use SchemeSel for selected non vacant tag */

static const char *fonts[] = {
    "Hack:size=10",
    "MesloLGS Nerd Font Mono:size=13",
};

// theme
#include "themes/minimal-mistakes.h"

static const char *colors[][3]      = {
    /*                        fg        bg      border */
    [SchemeNorm]            = {gray3,   black,  pink },
    [SchemeSel]             = {gray4,   green,   green  },
    [TabSel]                = {blue,    gray2,  black },
    [TabNorm]               = {gray3,   black,  black },
    [SchemeTag]             = {gray3,   black,  black },
    [SchemeTag1]            = {blue,    black,  black },
    [SchemeTag2]            = {red,     black,  black },
    [SchemeTag3]            = {orange,  black,  black },
    [SchemeTag4]            = {green,   black,  black },
    [SchemeTag5]            = {pink,    black,  black },
    [SchemeTag6]            = {blue,    black,  black },
    [SchemeTag7]            = {red,     black,  black },
    [SchemeTag8]            = {orange,  black,  black },
    [SchemeTag9]            = {green,   black,  black },
    [SchemeTag0]            = {pink,    black,  black },
    [SchemeBtnClk]          = {black,   red,    black },
    [SchemeLayout]          = {gray1,   black,  black },
    [SchemeBtnPrev]         = {green,   black,  black },
    [SchemeBtnNext]         = {yellow,  black,  black },
    [SchemeBtnClose]        = {red,     black,  black },
    [SchemeBtnClkMenu]      = {black,   blue,   black },
};

static const char *const autostart[] = {
    "autorandr", "--force", "--load", "flexiflow", NULL,
    "setxkbmap", "-option", "caps:escape", NULL,
    "xroot", "-r", "xmenu_desktop", NULL,
    "ckb-next", "--background", NULL,
    "touchpad-indicator", NULL,
    "youtube_subs", "-d", NULL,
    "clipmenu-daemon", NULL,
    "firewall-applet", NULL,
    "numlockx", "on", NULL,
    "blueman-applet", NULL,
    "toggle_conky", NULL,
    "pasystray", NULL,
    "nm-applet", NULL,
    "cbatticon", NULL,
    "dunst", NULL,
    NULL
};

/* tagging */
static char *tags[] = {"一", "二", "三", "四", "五", "六", "七", "八", "九" , "零"};
static const char *tagsalt[] = {"󰎤", "󰎧", "󰎪", "󰎭", "󰎱", "󰎳", "󰎶", "󰎹", "󰎼", "󰎡"};

static const int tagschemes[] = {
    SchemeTag1,
    SchemeTag2,
    SchemeTag3,
    SchemeTag4,
    SchemeTag5,
    SchemeTag6,
    SchemeTag7,
    SchemeTag8,
    SchemeTag9,
    SchemeTag0
};

static const unsigned int ulinepad      = 5; /* horizontal padding between the underline and tag */
static const unsigned int ulinestroke   = 2; /* thickness / height of the underline */
static const unsigned int ulinevoffset  = 0; /* how far above the bottom of the bar the line should appear */
static const int ulineall               = 0; /* 1 to show underline on all tags, 0 for just the active ones */

static const Rule rules[] = {
    /* xprop(1):
     *    WM_CLASS(STRING) = instance, class
     *    WM_NAME(STRING) = title
     */
    /* class                       instance      title                 tags mask       switchtotag       iscentered       isfloating   monitor   scratch key */
    //All workspaces
    {"Multimonitorlock-gui.py",    NULL,         NULL,                 0,              0,                1,               1,           -1,       0 },
    {"Pavucontrol",                NULL,         NULL,                 0,              0,                1,               1,           -1,       'P' },
    {"floatmenu",                  NULL,         NULL,                 0,              0,                1,               1,           -1,       0 },
    {"Yad",                        NULL,         NULL,                 0,              0,                1,               1,           -1,       0 },
    {"Peek",                       NULL,         NULL,                 0,              0,                1,               1,           -1,       0 },
    {"Snapper",                    NULL,         NULL,                 0,              0,                1,               1,           -1,       0 },
    {"scratchpad_terminal",        NULL,         NULL,                 0,              0,                1,               1,           -1,       'T' },
    {"scratchpad_btop",            NULL,         NULL,                 0,              0,                1,               1,           -1,       'B' },

    //Workspace 1 Monitor 0
    {"Alacritty",                  NULL,         NULL,                 1 << 0,         1,                0,                0,           0,       0 },
    {"kitty",                      NULL,         NULL,                 1 << 0,         1,                0,                0,           0,       0 },

    //Workspace 2 Monitor 0
    {"Code",                       NULL,         NULL,                 1 << 1,         1,                0,                0,           0,       0 },
    {"nvim",                       NULL,         NULL,                 1 << 1,         1,                0,                0,           0,       0 },
    {"jetbrains-studio",           NULL,         NULL,                 1 << 1,         1,                0,                0,           0,       0 },

    //Workspace 3 Monitor 0
    {"Nemo",                       NULL,         NULL,                 1 << 2,         1,                0,                0,           0,       0 },

    //Workspace 4 Monitor 0
    {"Ld-linux-x86-64.so.2",       NULL,         NULL,                 1 << 3,         1,                0,                0,           0,       0 }, // inkscape
    {"Gimp",                       NULL,         NULL,                 1 << 3,         1,                0,                0,           0,       0 },

    //Workspace 5 Monitor 0

    //Workspace 6 Monitor 1
    {"firefox",                    NULL,         NULL,                 1 << 5,         1,                0,                0,           0,       1 },
    {"Opera",                      NULL,         NULL,                 1 << 5,         1,                0,                0,           0,       1 },

    //Workspace 7 Monitor 1

    //Workspace 8 Monitor 1
    {"Steam",                      NULL,         NULL,                 1 << 7,         1,                0,                0,           0,       1 },
    {"RuneScape",                  NULL,         NULL,                 1 << 7,         1,                0,                0,           0,       1 },

    //Workspace 9 Monitor 1
    {"discord",                    NULL,         NULL,                 1 << 8,         1,                0,                0,           0,       1 },
    {"Hexchat",                    NULL,         NULL,                 1 << 8,         1,                0,                0,           0,       1 },

    //Workspace 10 Monitor 1
    {"PkgBrowser",                 NULL,         NULL,                 1 << 9,         1,                0,                0,           0,       1 },
    {NULL,                         NULL,         "Event Tester",       1 << 9,         1,                0,                0,           0,       1 }, // xev
    {NULL,                         NULL,         "lazygit",            1 << 9,         1,                0,                0,           0,       1 }, // lazygit

};

/* layout(s) */
static const float mfact     = 0.50; /* factor of master area size [0.05..0.95] */
static const int nmaster     = 1;    /* number of clients in master area */
static const int resizehints = 0;    /* 1 means respect size hints in tiled resizals */
static const int lockfullscreen = 1; /* 1 will force focus on the fullscreen window */
static const int attachdirection = 4;/* 0 default, 1 above, 2 aside, 3 below, 4 bottom, 5 top */

#define FORCE_VSPLIT 1  /* nrowgrid layout: force two clients to always split vertically */
#include "functions.h"

static const Layout layouts[] = {
    /* symbol     arrange function */
    {"[]=",      tile },    /* first entry is default */
    {"[M]",      monocle },
    {"[@]",      spiral },
    {"[D]",      dwindle },
    {"H[]",      deck },
    {"TTT",      bstack },
    {"===",      bstackhoriz },
    {"HHH",      grid },
    {"###",      nrowgrid },
    {"---",      horizgrid },
    {":::",      gaplessgrid },
    {"|M|",      centeredmaster },
    {">M>",      centeredfloatingmaster },
    {"><>",      NULL },    /* no layout function means floating behavior */
    {NULL,       NULL },
};

static const Taglayout taglayouts[] = {
    /*  tag        layout */
    {  1,        {.v = &layouts[11]} },
    {  2,        {.v = &layouts[0]} },
    {  3,        {.v = &layouts[0]} },
    {  4,        {.v = &layouts[0]} },
    {  5,        {.v = &layouts[0]} },
    {  6,        {.v = &layouts[0]} },
    {  7,        {.v = &layouts[0]} },
    {  8,        {.v = &layouts[0]} },
    {  9,        {.v = &layouts[0]} },
    {  10,        {.v = &layouts[0]} },
};

/* key definitions */
/* key definitions */
#define M Mod4Mask
#define A Mod1Mask
#define S ShiftMask
#define C ControlMask

#define TAGKEYS(CHAIN,KEY,TAG) \
  {M,         CHAIN,        KEY,      view,           {.ui = 1 << TAG} }, \
  {M|C,       CHAIN,        KEY,      toggleview,     {.ui = 1 << TAG} }, \
  {M|S,       CHAIN,        KEY,      tag,            {.ui = 1 << TAG} }, \
  {M|C|S,     CHAIN,        KEY,      toggletag,      {.ui = 1 << TAG} },

/* helper for spawning shell commands in the pre flexiflow-5.0 fashion */
#define SHCMD(cmd) {.v = (const char*[]){"/bin/sh", "-c", cmd, NULL} }

/* flexiflow commands */
static char repomenumon[2] = "0"; /* component of startmenucmd, manipulated in spawn() */
static const char *layoutmenucmd = "layoutmenu";
static const char *startmenucmd[] = {"menu", NULL};
static const char *passmenucmd[] = {"passmenu", NULL};
static const char *clipmenucmd[] = {"clipmenu_run", NULL};

/* Do not remove */
static const char *powermenucmd[] = {"powermenu_run", NULL};

/* repomenu commands */
static const char *filemanagercmd[] = {"nemo", NULL};
static const char *hypervisorcmd[] = {"gnome_boxes", NULL};
static const char *terminalcmd[] = {"alacritty", NULL};
static const char *browsercmd[] = {"opera", NULL};
static const char *editorcmd[] = {"code", NULL};

/* genreal commands*/
static const char *playernext[] = {"playerctl", "next", NULL};
static const char *playerprevious[] = {"playerctl", "previous", NULL};
static const char *playerpause[] = {"playerctl", "play-pause", NULL};

static const char *upvol[] = {"amixer", "set", "Master", "5%+", "unmute", NULL};
static const char *downvol[] = {"amixer", "set", "Master", "5%-", "unmute", NULL};
static const char *mutevol[] = {"amixer", "set", "Master", "toggle", NULL};

static const char *upbright[] = {"brightnessctl", "set", "5%+", NULL};
static const char *downbright[] = {"brightnessctl", "set", "5%-", NULL};

/*First arg only serves to match against key in rules*/
static const char *spcmd_term[] = {"T", "alacritty", "--title=scratchpad_terminal", "--class=scratchpad_terminal,scratchpad_terminal", NULL };
static const char *spcmd_btop[] = {"B", "st", "-n", "scratchpad_btop", "-c", "scratchpad_btop", "-t", "scratchpad_btop", "-e", "btop", NULL };
static const char *spcmd_pavu[] = {"P", "pavucontrol", NULL };

static const Launcher launchers[] = {
    /* command              name to display */
    {powermenucmd,         "" },
};

/* commands */
static Key keys[] = {
    /* modifier                         chain           key                                 function                argument */
    {M,                                 -1,             XK_n,                               togglealttag,           {0} },

    // Keybinds for XFKeys
    {0,                                 -1,             XF86XK_MonBrightnessUp,             spawn,                  {.v = upbright}},
    {0,                                 -1,             XF86XK_MonBrightnessDown,           spawn,                  {.v = downbright}},
    {0,                                 -1,             XF86XK_AudioLowerVolume,            spawn,                  {.v = downvol}},
    {0,                                 -1,             XF86XK_AudioMute,                   spawn,                  {.v = mutevol}},
    {0,                                 -1,             XF86XK_AudioRaiseVolume,            spawn,                  {.v = upvol}},
    {0,                                 -1,             XF86XK_AudioPlay,                   spawn,                  {.v = playerpause}},
    {0,                                 -1,             XF86XK_AudioPause,                  spawn,                  {.v = playerpause}},
    {0,                                 -1,             XF86XK_AudioNext,                   spawn,                  {.v = playernext}},
    {0,                                 -1,             XF86XK_AudioPrev,                   spawn,                  {.v = playerprevious}},

    // TheRepoClub Custom Keybinds.
    // Scratch Pads.
    {A,                                 -1,             XK_Return,                          togglescratch,          {.v = spcmd_term} },
    {A,                                 -1,             XK_b,                               togglescratch,          {.v = spcmd_btop} },
    {A,                                 -1,             XK_p,                               togglescratch,          {.v = spcmd_pavu} },

    // Spawns.
    {M,                                 -1,             XK_F1,                              spawn,                  {.v = clipmenucmd} },
    {M,                                 -1,             XK_F2,                              spawn,                  SHCMD("youtube_subs") },
    {M|S,                               -1,             XK_Return,                          spawn,                  {.v = filemanagercmd} },
    {M|C,                               -1,             XK_h,                               spawn,                  {.v = hypervisorcmd} },
    {M|C,                               -1,             XK_b,                               spawn,                  {.v = browsercmd} },
    {M|C,                               -1,             XK_e,                               spawn,                  {.v = editorcmd} },
    {M,                                 -1,             XK_p,                               spawn,                  {.v = passmenucmd} },
    {M,                                 -1,             XK_m,                               spawn,                  {.v = startmenucmd} },
    {M,                                 -1,             XK_Return,                          spawn,                  {.v = terminalcmd} },
    {M|S,                               -1,             XK_e,                               spawn,                  {.v = powermenucmd} },
    {M|C,                               -1,             XK_d,                               spawn,                  SHCMD("discord") },
    {M|C,                               -1,             XK_x,                               spawn,                  SHCMD("hexchat") },
    {M|C,                               -1,             XK_i,                               spawn,                  SHCMD("inkscape") },
    {M|C,                               -1,             XK_k,                               spawn,                  SHCMD("krita") },
    {M|C,                               XK_g,           XK_s,                               spawn,                  SHCMD("prime-run steam") },
    {M|C,                               XK_g,           XK_m,                               spawn,                  SHCMD("prime-run minecraft-launcher") },
    {M|C,                               XK_g,           XK_r,                               spawn,                  SHCMD("prime-run runescape-launcher") },
    {M|C,                               -1,             XK_p,                               spawn,                  SHCMD("pkgbrowser") },
    {M|S,                               -1,             XK_z,                               spawn,                  SHCMD("multimonitorlock-gui") },
    {M|A,                               -1,             XK_p,                               spawn,                  SHCMD("piper") },

    // screenshot fullscreen and cropped
    {M,                                 XK_Print,       XK_w,                               spawn,                  SHCMD("win-shot -w" ) },
    {M,                                 XK_Print,       XK_s,                               spawn,                  SHCMD("win-shot -s" ) },
    {M,                                 XK_Print,       XK_f,                               spawn,                  SHCMD("win-shot -f" ) },

    // toggle stuff
    {M,                                 -1,             XK_b,                               togglebar,              {0} },
    {M,                                 -1,             XK_s,                               togglesystray,          {0} },
    {M,                                 -1,             XK_g,                               togglegaps,             {0} },

    {M|S,                               -1,             XK_space,                           togglefloating,         {0} },
    {M,                                 -1,             XK_f,                               togglefullscr,          {0} },

    {M|C,                               -1,             XK_w,                               tabmode,                {-1} },
    {M,                                 -1,             XK_j,                               focusstack,             {.i = +1} },
    {M,                                 -1,             XK_k,                               focusstack,             {.i = -1} },
    {M,                                 -1,             XK_i,                               incnmaster,             {.i = +1} },
    {M,                                 -1,             XK_u,                               incnmaster,             {.i = -1} },

    // change m,cfact sizes
    {M,                                 -1,             XK_h,                               setmfact,               {.f = -0.05} },
    {M,                                 -1,             XK_l,                               setmfact,               {.f = +0.05} },
    {M|S,                               -1,             XK_h,                               setcfact,               {.f = +0.25} },
    {M|S,                               -1,             XK_l,                               setcfact,               {.f = -0.25} },
    {M|S,                               -1,             XK_o,                               setcfact,               {.f =  0.00} },

    {M|S,                               -1,             XK_j,                               movestack,              {.i = +1} },
    {M|S,                               -1,             XK_k,                               movestack,              {.i = -1} },
    {M|S,                               -1,             XK_Return,                          zoom,                   {0} },
    {M,                                 -1,             XK_Tab,                             view,                   {0} },

    // overall gaps
    {M|S,                               -1,             XK_u,                               incrgaps,               {.i = +1} },
    {M|C|S,                             -1,             XK_u,                               incrgaps,               {.i = -1} },

    // inner gaps
    {M|S,                               -1,             XK_i,                               incrigaps,              {.i = +1} },
    {M|C|S,                             -1,             XK_i,                               incrigaps,              {.i = -1} },

    // outer gaps
    {M|C,                               -1,             XK_o,                               incrogaps,              {.i = +1} },
    {M|C|S,                             -1,             XK_o,                               incrogaps,              {.i = -1} },

    // inner+outer hori, vert gaps
    {M|C,                               -1,             XK_KP_Up,                           incrihgaps,             {.i = +1} },
    {M|C|S,                             -1,             XK_KP_Up,                           incrihgaps,             {.i = -1} },
    {M|C,                               -1,             XK_KP_Right,                        incrivgaps,             {.i = +1} },
    {M|C|S,                             -1,             XK_KP_Right,                        incrivgaps,             {.i = -1} },
    {M|C,                               -1,             XK_KP_Down,                         incrohgaps,             {.i = +1} },
    {M|C|S,                             -1,             XK_KP_Down,                         incrohgaps,             {.i = -1} },
    {M|C,                               -1,             XK_KP_Left,                         incrovgaps,             {.i = +1} },
    {M|C|S,                             -1,             XK_KP_Left,                         incrovgaps,             {.i = -1} },

    {M|C|S,                             -1,             XK_d,                               defaultgaps,            {0} },

    // layout
    {M,                                 -1,             XK_space,                           setlayout,              {0} },
    {M|C,                               -1,             XK_comma,                           cyclelayout,            {.i = -1} },
    {M|C,                               -1,             XK_period,                          cyclelayout,            {.i = +1} },

    {M,                                 -1,             XK_comma,                           focusmon,               {.i = -1} },
    {M,                                 -1,             XK_period,                          focusmon,               {.i = +1} },
    {M|S,                               -1,             XK_comma,                           tagmon,                 {.i = -1} },
    {M|S,                               -1,             XK_period,                          tagmon,                 {.i = +1} },

    // change border size
    {M|S,                               -1,             XK_minus,                           setborderpx,            {.i = -1} },
    {M|S,                               -1,             XK_equal,                           setborderpx,            {.i = +1} },
    {M|S,                               -1,             XK_d,                               setborderpx,            {.i = default_border} },

    // kill window
    {M,                                 -1,             XK_q,                               killclient,             {0} },

    // restart
    {M|S,                               -1,             XK_r,                               quit,                   {1} },
    {M|S,                               -1,             XK_q,                               quit,                   {0} },

    // hide & restore windows
    {M,                                 -1,             XK_w,                               hidewin,                {0} },
    {M|S,                               -1,             XK_w,                               restorewin,             {0} },

    TAGKEYS(                            -1,             XK_1,                               0)
    TAGKEYS(                            -1,             XK_2,                               1)
    TAGKEYS(                            -1,             XK_3,                               2)
    TAGKEYS(                            -1,             XK_4,                               3)
    TAGKEYS(                            -1,             XK_5,                               4)
    TAGKEYS(                            -1,             XK_6,                               5)
    TAGKEYS(                            -1,             XK_7,                               6)
    TAGKEYS(                            -1,             XK_8,                               7)
    TAGKEYS(                            -1,             XK_9,                               8)
    TAGKEYS(                            -1,             XK_0,                               9)
};

/* button definitions */
/* click can be ClkTagBar, ClkLtSymbol, ClkStatusText, ClkWinTitle, ClkClientWin, or ClkRootWin */
static Button buttons[] = {
    /* click                            event mask      button                              function                argument */
    {ClkMenuButton,                     0,              Button1,                            spawn,                  {.v = startmenucmd } },
    {ClkLtSymbol,                       0,              Button1,                            setlayout,              {0} },
    {ClkLtSymbol,                       0,              Button3,                            setlayout,              {.v = &layouts[2]} },
    {ClkLtSymbol,                       0,              Button2,                            layoutmenu,             {0} },
    {ClkWinTitle,                       0,              Button2,                            zoom,                   {0} },
    {ClkStatusText,                     0,              Button2,                            spawn,                  {.v = terminalcmd} },

    /* Keep movemouse? */
    {ClkClientWin,                      M,              Button1,                            movemouse,              {0} },

    /* placemouse options, choose which feels more natural:
    *    0 - tiled position is relative to mouse cursor
    *    1 - tiled position is relative to window center
    *    2 - mouse pointer warps to window center
    *
    * The moveorplace uses movemouse or placemouse depending on the floating state
    * of the selected client. Set up individual keybindings for the two if you want
    * to control these separately (i.e. to retain the feature to move a tiled window
    * into a floating position).
    */
    {ClkClientWin,                      M,              Button1,                            moveorplace,            {.i = 0} },
    {ClkClientWin,                      M,              Button2,                            togglefloating,         {0} },
    {ClkClientWin,                      M,              Button3,                            resizemouse,            {0} },
    {ClkClientWin,                      A,              Button1,                            dragmfact,              {0} },
    {ClkClientWin,                      A,              Button3,                            dragcfact,              {0} },
    {ClkTagBar,                         0,              Button1,                            view,                   {0} },
    {ClkTagBar,                         0,              Button3,                            toggleview,             {0} },
    {ClkTagBar,                         M,              Button1,                            tag,                    {0} },
    {ClkTagBar,                         M,              Button3,                            toggletag,              {0} },
    {ClkTabBar,                         0,              Button1,                            focuswin,               {0} },
    {ClkTabBar,                         0,              Button1,                            focuswin,               {0} },
    {ClkTabPrev,                        0,              Button1,                            movestack,              {.i = -1} },
    {ClkTabNext,                        0,              Button1,                            movestack,              {.i = +1} },
    {ClkTabClose,                       0,              Button1,                            killclient,             {0} },
};

/// Cursor
pub enum Cur {
    Normal,
    Resize,
    Move,
    Last,
}

/// Color schemes
pub enum Scheme {
    Norm,
    Sel,
    /// the bar's layout symbol
    Layout,
    /// the bar's power button
    Power,
    /// the bar's menu button, left of the tags
    Menu,
    /// the focused window's title in the bar
    Title,
}

/// Clicks
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Clk {
    TagBar,
    LtSymbol,
    StatusText,
    WinTitle,
    ClientWin,
    RootWin,
    /// `Config::powermenu`'s symbol, right after the layout symbol
    PowerButton,
    /// `Config::menubutton`, left of the tags
    MenuButton,
    Last,
}

/// Default atoms
pub enum WM {
    Protocols,
    Delete,
    State,
    TakeFocus,
    /// `crate::SETLAYOUT_ATOM`, for `FlexiFlow -l`
    SetLayout,
    Last,
}

/// EWMH atoms
pub enum Net {
    Supported,
    WMName,
    WMState,
    WMCheck,
    SystemTray,
    SystemTrayOP,
    SystemTrayOrientation,
    SystemTrayOrientationHorz,
    WMFullscreen,
    ActiveWindow,
    WMWindowType,
    WMWindowTypeDialog,
    ClientList,
    Last,
}

pub enum XEmbed {
    Manager,
    XEmbed,
    XEmbedInfo,
    Last,
}

/// Clr scheme index
pub enum Col {
    Fg,
    Bg,
    Border,
}

//! The layout and power menus: the lines go into the dmenu-style
//! `config.menu` program on a thread of their own, and what's picked comes
//! back to the main loop over the IPC socket, like any other request.
use std::{
    io::Write as _,
    process::{Command, Stdio},
};

use flexiflow_river::{
    config::{Arg, Func, Layout, PowerMenuItem},
    ipc::{self, Request},
};

use crate::wm::Wm;

/// One menu line: `text` behind fleximenu's `:<color> ` prefix when
/// there's a `color`, which boxes the text up to the next space (a layout
/// symbol, a label's icon) in that color while it's selected.
fn menu_line(color: &str, text: &str) -> String {
    if color.is_empty() { format!("{text}\n") } else { format!(":{color} {text}\n") }
}

fn layout_menu_text(layouts: &[Layout]) -> String {
    layouts
        .iter()
        .map(|l| {
            let desc = crate::actions::layout_desc(l.arrange.as_deref());
            menu_line(&l.color, &format!("{} {desc}", l.symbol))
        })
        .collect()
}

fn power_menu_text(items: &[PowerMenuItem]) -> String {
    items.iter().map(|i| menu_line(&i.color, &i.label)).collect()
}

/// Run `menu` with `-p prompt -l <lines>` on a thread, then `picked` with
/// the index of the line picked, if one of them was.
fn open(menu: &[String], prompt: &str, lines: String, picked: impl FnOnce(usize) + Send + 'static) {
    let Some((cmd, args)) = menu.split_first() else { return };
    let mut c = Command::new(cmd);
    let n = lines.lines().count().to_string();
    c.args(args).args(["-p", prompt, "-l", &n]);
    c.stdin(Stdio::piped()).stdout(Stdio::piped());
    std::thread::spawn(move || {
        let mut child = match c.spawn() {
            Ok(child) => child,
            Err(e) => return log::warn!("menu: {e}"),
        };
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(lines.as_bytes());
        }
        let Ok(out) = child.wait_with_output() else { return };
        let choice = String::from_utf8_lossy(&out.stdout);
        let choice = choice.trim_end_matches('\n');
        if let Some(i) = lines.lines().position(|l| l == choice) {
            picked(i);
        }
    });
}

fn send(func: &str, arg: Arg) {
    let req = Request::Action { output: None, func: Func(func.to_string()), arg };
    if let Err(e) = ipc::send(&req) {
        log::warn!("menu: can't reach flexiflow-river: {e}");
    }
}

pub fn layoutmenu(wm: &Wm) {
    let lines = layout_menu_text(&wm.config.layouts);
    open(&wm.config.menu, "Layout", lines, |i| send("setlayout", Arg::L(Some(i))));
}

pub fn powermenu(wm: &Wm) {
    let items = wm.config.powermenu.items.clone();
    let lines = power_menu_text(&items);
    open(&wm.config.menu, "Power", lines, move |i| {
        send("spawn", Arg::V(vec!["sh".into(), "-c".into(), items[i].command.clone()]));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_menu_text_puts_each_color_before_its_symbol() {
        let layout = |symbol: &str, arrange: Option<&str>, color: &str| Layout {
            symbol: symbol.into(),
            arrange: arrange.map(Into::into),
            color: color.into(),
        };
        let layouts = vec![
            layout("[]=", Some("tile"), "b"),
            layout("[M]", Some("monocle"), ""),
            layout("><>", None, "h"),
        ];
        assert_eq!(
            layout_menu_text(&layouts),
            ":b []= Tiled Layout\n[M] Monocle Layout\n:h ><> NULL Layout\n"
        );
    }
}

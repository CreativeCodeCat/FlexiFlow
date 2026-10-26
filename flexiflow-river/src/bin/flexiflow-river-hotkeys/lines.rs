//! What the popup lists: every described key binding under its group's
//! heading, filtered by what's typed and split into columns. The same
//! rules as the X11 FlexiFlow's popup.
use flexiflow_river::config::{Key, keysyms::KEYSYMS};

/// A line of the popup: a group's heading, or one of its keys with its
/// description. Both carry the group's position, which picks its color.
#[derive(Debug, PartialEq)]
pub enum HotkeyLine<'a> {
    Group(usize, &'a str),
    Key(usize, String, &'a str),
}

/// A group's rows while they're collected: each row's modifiers,
/// description, and the names of the keys that share them.
type Rows<'a> = Vec<(u32, &'a str, Vec<String>)>;

/// Every described key under its group's heading, the groups in the
/// order `order` lists them and then in the order they're first bound,
/// and keys without a group last under "other". Keys that share their
/// group, modifiers and description, like the tag keys, share a line:
/// `Super+0-9`.
pub fn hotkey_lines<'a>(keys: &'a [Key], order: &[String]) -> Vec<HotkeyLine<'a>> {
    let mut groups: Vec<(&str, Rows)> = Vec::new();
    let described = keys.iter().filter(|k| !k.desc.is_empty());
    for key in described.clone().filter(|k| !k.group.is_empty()).chain(described.filter(|k| k.group.is_empty())) {
        let group = if key.group.is_empty() { "other" } else { &key.group };
        let rows = match groups.iter().position(|(g, _)| *g == group) {
            Some(i) => &mut groups[i].1,
            None => {
                groups.push((group, Vec::new()));
                &mut groups.last_mut().unwrap().1
            }
        };
        let name = keysym_name(key.keysym);
        match rows.iter_mut().find(|(m, d, _)| *m == key.mod_ && *d == key.desc) {
            Some((_, _, names)) => names.push(name),
            None => rows.push((key.mod_, &key.desc, vec![name])),
        }
    }
    // stable, so unlisted groups keep their order after the listed ones
    groups.sort_by_key(|(g, _)| order.iter().position(|o| o == g).unwrap_or(order.len()));
    let mut lines = Vec::new();
    for (i, (group, rows)) in groups.into_iter().enumerate() {
        lines.push(HotkeyLine::Group(i, group));
        for (mods, desc, names) in rows {
            let keys = match names.as_slice() {
                [one] => one.clone(),
                [a, b] => format!("{a}/{b}"),
                // the tag keys, 1 to 9 then 0, are all ten digits
                all if all.len() == 10 && all.iter().all(|n| n.len() == 1 && n.as_bytes()[0].is_ascii_digit()) => {
                    "0-9".to_string()
                }
                [first, .., last] => format!("{first}-{last}"),
                [] => unreachable!(),
            };
            lines.push(HotkeyLine::Key(i, format!("{}{keys}", mod_names(mods)), desc));
        }
    }
    lines
}

/// The lines whose keys, description, group or heading contain `query`,
/// ignoring case, under their headings; all of them for an empty query.
/// A group whose name or heading matches keeps all its keys.
pub fn hotkey_filter<'a, 'b>(lines: &'b [HotkeyLine<'a>], query: &str, labels: &[String]) -> Vec<&'b HotkeyLine<'a>> {
    let query = query.trim().to_lowercase();
    let has = |s: &str| s.to_lowercase().contains(&query);
    let mut shown = Vec::new();
    let mut heading = None;
    let mut whole_group = false;
    for line in lines {
        match line {
            HotkeyLine::Group(i, name) => {
                heading = Some(line);
                whole_group = has(name) || labels.get(*i).is_some_and(|l| has(l));
            }
            HotkeyLine::Key(_, keys, desc) => {
                if whole_group || has(keys) || has(desc) {
                    if let Some(h) = heading.take() {
                        shown.push(h);
                    }
                    shown.push(line);
                }
            }
        }
    }
    shown
}

/// `lines` split into columns of about `per_col` lines, never ending a
/// column on a heading: it moves to the top of the next one instead.
pub fn hotkey_columns<'a, 'b>(lines: &[&'b HotkeyLine<'a>], per_col: usize) -> Vec<Vec<&'b HotkeyLine<'a>>> {
    let mut columns: Vec<Vec<&HotkeyLine>> = vec![Vec::new()];
    for &line in lines {
        let col = columns.last_mut().unwrap();
        let heading_at_bottom = matches!(line, HotkeyLine::Group(..)) && col.len() + 1 >= per_col;
        if col.len() >= per_col || (heading_at_bottom && !col.is_empty()) {
            columns.push(Vec::new());
        }
        columns.last_mut().unwrap().push(line);
    }
    columns
}

/// `keys`' modifiers and key, one per badge: `Super+Shift+J` is `Super`,
/// `Shift` and `J`. A `+` at the end is the key itself.
pub fn hotkey_parts(keys: &str) -> Vec<&str> {
    match keys.strip_suffix("++") {
        Some(mods) => mods.split('+').chain(["+"]).collect(),
        None => keys.split('+').filter(|p| !p.is_empty()).collect(),
    }
}

/// `mods` as `"Super+Ctrl+Alt+Shift+"`, leaving out the ones not held.
fn mod_names(mods: u32) -> String {
    [(64, "Super"), (4, "Ctrl"), (8, "Alt"), (1, "Shift")]
        .iter()
        .filter(|(mask, _)| mods & mask != 0)
        .map(|(_, name)| format!("{name}+"))
        .collect()
}

/// A short name for `keysym`: the symbol itself for punctuation, letters
/// in capitals and media keys without their `XF86` prefix.
fn keysym_name(keysym: u32) -> String {
    let Some((name, _)) = KEYSYMS.iter().find(|(_, v)| *v == keysym) else {
        return format!("{keysym:#x}");
    };
    let name = name.strip_prefix("XF86XK_").or_else(|| name.strip_prefix("XK_")).unwrap_or(name);
    match name {
        "comma" => ",".into(),
        "period" => ".".into(),
        "slash" => "/".into(),
        "bracketleft" => "[".into(),
        "bracketright" => "]".into(),
        "space" => "Space".into(),
        s if s.len() == 1 => s.to_uppercase(),
        s => s.to_string(),
    }
}

/// `name` with its first letter in capitals.
pub fn capitalized(name: &str) -> String {
    let mut chars = name.chars();
    chars.next().map_or_else(String::new, |c| c.to_uppercase().chain(chars).collect())
}

#[cfg(test)]
mod tests {
    use flexiflow_river::config::{Arg, Func};

    use super::*;

    const SUPER: u32 = 64;
    const SHIFT: u32 = 1;
    const CTRL: u32 = 4;

    fn described(mod_: u32, keysym: u32, desc: &str, group: &str) -> Key {
        Key {
            mod_,
            keysym,
            func: Func("view".into()),
            arg: Arg::I(0),
            desc: desc.into(),
            group: group.into(),
        }
    }

    #[test]
    fn lines_name_the_described_keys_under_their_groups() {
        let keys = vec![
            described(SUPER, 0xff0d, "Open a terminal", "launcher"),
            described(SUPER | SHIFT, 0x6a, "", "client"),
            described(SUPER | CTRL, 0x2c, "Previous layout", "layout"),
            described(SUPER, 0x6d, "Run a program", "launcher"),
        ];
        assert_eq!(
            hotkey_lines(&keys, &[]),
            [
                HotkeyLine::Group(0, "launcher"),
                HotkeyLine::Key(0, "Super+Return".into(), "Open a terminal"),
                HotkeyLine::Key(0, "Super+M".into(), "Run a program"),
                HotkeyLine::Group(1, "layout"),
                HotkeyLine::Key(1, "Super+Ctrl+,".into(), "Previous layout"),
            ]
        );
    }

    #[test]
    fn lines_join_keys_and_put_ungrouped_ones_last() {
        let keys = vec![
            described(0, 0x1008ff14, "Play", ""),
            described(SUPER, 0x31, "View tag", "tag"),
            described(SUPER, 0x32, "View tag", "tag"),
            described(SUPER, 0x30, "View tag", "tag"),
        ];
        assert_eq!(
            hotkey_lines(&keys, &[]),
            [
                HotkeyLine::Group(0, "tag"),
                HotkeyLine::Key(0, "Super+1-0".into(), "View tag"),
                HotkeyLine::Group(1, "other"),
                HotkeyLine::Key(1, "AudioPlay".into(), "Play"),
            ]
        );
    }

    #[test]
    fn lines_put_the_listed_groups_first() {
        let keys = vec![
            described(SUPER, 0x61, "A", "apps"),
            described(SUPER, 0x62, "B", "tag"),
            described(SUPER, 0x63, "C", "flexiflow"),
        ];
        let order = ["flexiflow".to_string(), "tag".to_string()];
        let groups: Vec<_> = hotkey_lines(&keys, &order)
            .into_iter()
            .filter_map(|l| match l {
                HotkeyLine::Group(_, g) => Some(g),
                HotkeyLine::Key(..) => None,
            })
            .collect();
        assert_eq!(groups, ["flexiflow", "tag", "apps"]);
    }

    #[test]
    fn all_ten_tag_keys_are_a_digit_range() {
        let keys: Vec<_> = (0x30..=0x39).map(|k| described(SUPER, k, "View tag", "tag")).collect();
        assert_eq!(hotkey_lines(&keys, &[])[1], HotkeyLine::Key(0, "Super+0-9".into(), "View tag"));
    }

    #[test]
    fn parts_split_the_modifiers_from_the_key() {
        assert_eq!(hotkey_parts("Super+Shift+J"), ["Super", "Shift", "J"]);
        assert_eq!(hotkey_parts("AudioPlay"), ["AudioPlay"]);
        assert_eq!(hotkey_parts("Super++"), ["Super", "+"]);
    }

    #[test]
    fn filtering_keeps_matching_keys_under_their_headings() {
        let lines = [
            HotkeyLine::Group(0, "media"),
            HotkeyLine::Key(0, "AudioRaiseVolume".into(), "Volume up"),
            HotkeyLine::Key(0, "AudioPlay".into(), "Play/pause"),
            HotkeyLine::Group(1, "gaps"),
            HotkeyLine::Key(1, "Super+G".into(), "Toggle gaps"),
        ];
        let labels = ["Media".to_string(), "Gaps".to_string()];
        assert_eq!(hotkey_filter(&lines, "VOLUME", &labels), [&lines[0], &lines[1]]);
        // a group's heading matching keeps all its keys
        assert_eq!(hotkey_filter(&lines, "gap", &labels), [&lines[3], &lines[4]]);
        assert_eq!(hotkey_filter(&lines, "", &labels).len(), lines.len());
        assert!(hotkey_filter(&lines, "nothing", &labels).is_empty());
    }

    #[test]
    fn columns_move_a_heading_off_the_bottom() {
        let key = |d| HotkeyLine::Key(0, String::new(), d);
        let lines = [HotkeyLine::Group(0, "a"), key("1"), key("2"), HotkeyLine::Group(1, "b"), key("3")];
        let lines: Vec<_> = lines.iter().collect();
        let columns = hotkey_columns(&lines, 4);
        assert_eq!(columns.len(), 2);
        assert_eq!(columns[0].len(), 3);
        assert_eq!(columns[1][0], &HotkeyLine::Group(1, "b"));
    }
}

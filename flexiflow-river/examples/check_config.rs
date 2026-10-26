//! Load the config flexiflow-river would use and summarize it, to check
//! an edit before restarting: `cargo run --example check_config`.
use flexiflow_river::config::Config;

fn main() {
    let path = Config::path().expect("no config directory");
    println!("{}", path.display());
    match Config::from_lua(&path) {
        Ok(c) if std::env::args().nth(1).as_deref() == Some("autostart") => {
            for argv in &c.autostart {
                println!("{argv:?}");
            }
        }
        Ok(c) if std::env::args().nth(1).as_deref() == Some("spawns") => {
            for k in c.keys.iter().filter(|k| k.func.0 == "spawn") {
                println!("{:?}", k.arg);
            }
        }
        Ok(c) => println!(
            "ok: {} keys, {} buttons, {} rules, {} layouts, {} tags, {} scratchpads, {} autostart",
            c.keys.len(),
            c.buttons.len(),
            c.rules.len(),
            c.layouts.len(),
            c.tags.len(),
            c.scratchpads.len(),
            c.autostart.len()
        ),
        Err(e) => {
            println!("error: {e}");
            std::process::exit(1);
        }
    }
}

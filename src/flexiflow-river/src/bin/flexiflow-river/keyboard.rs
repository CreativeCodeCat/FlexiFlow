//! The `keyboard` config section, applied with river-xkb-config-v1: the
//! keymap is compiled here from the layout, variant, model and options,
//! as `setxkbmap` would, handed to river, and set on every keyboard once
//! river has accepted it - keyboards plugged in later too.
use std::{
    io::Write as _,
    os::fd::{AsFd as _, FromRawFd as _, OwnedFd},
};

use flexiflow_river::{
    config::Keyboard,
    protocols::xkb_config::{
        river_xkb_config_v1::{self, KeymapFormat, RiverXkbConfigV1},
        river_xkb_keyboard_v1::{self, RiverXkbKeyboardV1},
        river_xkb_keymap_v1::{self, RiverXkbKeymapV1},
    },
};
use wayland_client::{Connection, Dispatch, QueueHandle, event_created_child};

use crate::wm::Wm;

#[derive(Default)]
pub struct Keyboards {
    pub config: Option<RiverXkbConfigV1>,
    keyboards: Vec<RiverXkbKeyboardV1>,
    /// the configured keymap, once river has accepted it
    keymap: Option<RiverXkbKeymapV1>,
}

/// `keymap` as text in a sealed memfd, as river wants it.
fn keymap_fd(keymap: &str) -> std::io::Result<OwnedFd> {
    let fd = unsafe {
        let fd = libc::memfd_create(c"flexiflow-keymap".as_ptr(), libc::MFD_CLOEXEC | libc::MFD_ALLOW_SEALING);
        if fd < 0 {
            return Err(std::io::Error::last_os_error());
        }
        OwnedFd::from_raw_fd(fd)
    };
    let mut file = std::fs::File::from(fd);
    file.write_all(keymap.as_bytes())?;
    file.write_all(&[0])?;
    let fd = OwnedFd::from(file);
    let seals = libc::F_SEAL_SEAL | libc::F_SEAL_SHRINK | libc::F_SEAL_GROW | libc::F_SEAL_WRITE;
    unsafe { libc::fcntl(std::os::fd::AsRawFd::as_raw_fd(&fd), libc::F_ADD_SEALS, seals) };
    Ok(fd)
}

/// Compile `kb` the way `setxkbmap` would.
fn compile(kb: &Keyboard) -> Option<String> {
    use xkbcommon::xkb;
    let ctx = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
    let options = (!kb.options.is_empty()).then(|| kb.options.clone());
    let keymap =
        xkb::Keymap::new_from_names(&ctx, "", &kb.model, &kb.layout, &kb.variant, options, xkb::KEYMAP_COMPILE_NO_FLAGS)?;
    Some(keymap.get_as_string(xkb::KEYMAP_FORMAT_TEXT_V1))
}

impl Wm {
    /// Ask river for the configured keymap. Called once, when the
    /// xkb-config global has been bound.
    pub fn request_keymap(&mut self) {
        let (Some(kb), Some(xkb)) = (self.config.keyboard.clone(), self.keyboards.config.clone()) else { return };
        let Some(text) = compile(&kb) else {
            log::error!("keyboard: no such layout {:?} (variant {:?}, options {:?})", kb.layout, kb.variant, kb.options);
            return;
        };
        match keymap_fd(&text) {
            Ok(fd) => {
                xkb.create_keymap(fd.as_fd(), KeymapFormat::TextV1, &self.qh, ());
            }
            Err(e) => log::error!("keyboard: can't pass the keymap to river: {e}"),
        }
    }

    fn configure_keyboard(&self, kb: &RiverXkbKeyboardV1) {
        let Some(keymap) = &self.keyboards.keymap else { return };
        kb.set_keymap(keymap);
        if self.config.keyboard.as_ref().is_some_and(|k| k.numlock) {
            kb.numlock_enable();
        }
    }
}

impl Dispatch<RiverXkbConfigV1, ()> for Wm {
    fn event(
        wm: &mut Self,
        _: &RiverXkbConfigV1,
        event: river_xkb_config_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let river_xkb_config_v1::Event::XkbKeyboard { id } = event {
            wm.configure_keyboard(&id);
            wm.keyboards.keyboards.push(id);
        }
    }

    event_created_child!(Wm, RiverXkbConfigV1, [
        river_xkb_config_v1::EVT_XKB_KEYBOARD_OPCODE => (RiverXkbKeyboardV1, ()),
    ]);
}

impl Dispatch<RiverXkbKeymapV1, ()> for Wm {
    fn event(
        wm: &mut Self,
        keymap: &RiverXkbKeymapV1,
        event: river_xkb_keymap_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            river_xkb_keymap_v1::Event::Success => {
                wm.keyboards.keymap = Some(keymap.clone());
                for kb in wm.keyboards.keyboards.clone() {
                    wm.configure_keyboard(&kb);
                }
            }
            river_xkb_keymap_v1::Event::Failure { error_msg } => {
                log::error!("keyboard: river refused the keymap: {error_msg}");
                keymap.destroy();
            }
            _ => {}
        }
    }
}

impl Dispatch<RiverXkbKeyboardV1, ()> for Wm {
    fn event(
        wm: &mut Self,
        kb: &RiverXkbKeyboardV1,
        event: river_xkb_keyboard_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let river_xkb_keyboard_v1::Event::Removed = event {
            wm.keyboards.keyboards.retain(|k| k != kb);
            kb.destroy();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layouts_compile_like_setxkbmap() {
        let gb = Keyboard { layout: "gb".into(), options: "caps:escape".into(), ..Default::default() };
        let text = compile(&gb).expect("gb should compile");
        // the symbols section is named after what went into it
        assert!(text.contains("xkb_symbols \"pc_gb_"), "not the gb layout");
        assert!(text.contains("capslock(escape)"), "caps:escape missing");
        let nonsense = Keyboard { layout: "not-a-layout".into(), ..Default::default() };
        assert!(compile(&nonsense).is_none());
    }
}

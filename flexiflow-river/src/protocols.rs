//! Client bindings for river 0.5's own protocols, generated from the
//! XML in `protocols/`.
#![allow(dead_code, non_upper_case_globals, non_camel_case_types, clippy::all)]

pub mod window_management {
    use wayland_client;
    use wayland_client::protocol::*;

    pub mod __interfaces {
        use wayland_client::protocol::__interfaces::*;
        wayland_scanner::generate_interfaces!("protocols/river-window-management-v1.xml");
    }
    use self::__interfaces::*;

    wayland_scanner::generate_client_code!("protocols/river-window-management-v1.xml");
}

pub mod xkb_bindings {
    use super::window_management::*;
    use wayland_client;

    pub mod __interfaces {
        use super::super::window_management::__interfaces::*;
        wayland_scanner::generate_interfaces!("protocols/river-xkb-bindings-v1.xml");
    }
    use self::__interfaces::*;

    wayland_scanner::generate_client_code!("protocols/river-xkb-bindings-v1.xml");
}

pub mod layer_shell {
    use super::window_management::*;
    use wayland_client;

    pub mod __interfaces {
        use super::super::window_management::__interfaces::*;
        wayland_scanner::generate_interfaces!("protocols/river-layer-shell-v1.xml");
    }
    use self::__interfaces::*;

    wayland_scanner::generate_client_code!("protocols/river-layer-shell-v1.xml");
}

pub mod input_management {
    use wayland_client;
    use wayland_client::protocol::*;

    pub mod __interfaces {
        use wayland_client::protocol::__interfaces::*;
        wayland_scanner::generate_interfaces!("protocols/river-input-management-v1.xml");
    }
    use self::__interfaces::*;

    wayland_scanner::generate_client_code!("protocols/river-input-management-v1.xml");
}

pub mod xkb_config {
    use super::input_management::*;
    use wayland_client;

    pub mod __interfaces {
        use super::super::input_management::__interfaces::*;
        wayland_scanner::generate_interfaces!("protocols/river-xkb-config-v1.xml");
    }
    use self::__interfaces::*;

    wayland_scanner::generate_client_code!("protocols/river-xkb-config-v1.xml");
}

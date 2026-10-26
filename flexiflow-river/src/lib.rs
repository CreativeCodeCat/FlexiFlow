//! Shared by the window manager (`flexiflow-river`), its bar
//! (`flexiflow-river-bar`) and the status program (`flexiflow-river-status`):
//! everything from `flexiflow-common`, under the same names as before, and
//! the river protocols.
pub use flexiflow_common::{canvas, config, font, ipc, layouts, status};
pub mod protocols;

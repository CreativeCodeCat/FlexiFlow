//! What FlexiFlow's X11 build and its river build share: the layout
//! algorithms, the river build's Lua config, the IPC socket format,
//! status text and drawing (fonts and canvases).
pub mod canvas;
pub mod config;
pub mod font;
pub mod ipc;
pub mod layouts;
pub mod status;

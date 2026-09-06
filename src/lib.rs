//! The cockpit library: every module the binary drives, exposed so the test
//! suite and the headless surfaces can reach them.

// `AppState` carries some eighty fields, and a test fixture sets three of them
// before handing the rest their defaults:
//
//     let mut state = AppState::default();
//     state.active_pane = Pane::Mannies;
//
// Clippy would rather see struct-update syntax. On a struct this wide that
// reads worse, not better, and it is the shape every fixture in the suite
// already uses — so the lint is wrong here rather than the code, and saying so
// once beats rewriting hundreds of lines of setup or leaving a third of the
// repository unlinted (issue #349). Test code only: the lint still applies to
// the library itself, where a wide-struct fixture is not the idiom.
#![cfg_attr(test, allow(clippy::field_reassign_with_default))]

pub mod api;
pub mod app;
pub mod config;
pub mod diaglog;
pub mod headless;
pub mod input;
pub mod notify;
pub mod preflight;
pub mod store;
pub mod termbg;
pub mod ui;
pub mod update;

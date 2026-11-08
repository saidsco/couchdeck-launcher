//! CouchDeck – a controller-first fullscreen game launcher.
//!
//! The crate is split into a library (this file) and a thin binary so that the
//! non-UI parts can be unit tested without opening a window.

pub mod activation;
pub mod app;
pub mod artwork;
pub mod controller;
pub mod desktop;
pub mod i18n;
pub mod instance;
pub mod launch;
pub mod logging;
pub mod model;
pub mod paths;
pub mod security;
pub mod session;
pub mod sound;
pub mod storage;
pub mod system;
pub mod timekeeping;
pub mod ui;

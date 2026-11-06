//! CouchDeck – a controller-first fullscreen game launcher.
//!
//! The crate is split into a library (this file) and a thin binary so that the
//! non-UI parts can be unit tested without opening a window.

pub mod app;
pub mod artwork;
pub mod controller;
pub mod desktop;
pub mod launch;
pub mod logging;
pub mod model;
pub mod paths;
pub mod power;
pub mod security;
pub mod storage;
pub mod ui;

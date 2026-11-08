//! Minimal xdg-activation-v1 client for an existing winit/Wayland surface.
//!
//! eframe 0.32 does not expose an API for applying an incoming activation
//! token to an already-created window. Winit's public API only exposes tokens
//! at window creation or for spawning child processes. The raw Wayland handles
//! exposed by eframe are stable for the lifetime of the window, so this module
//! opens a guest connection to the same `wl_display`, discovers
//! `xdg_activation_v1`, and sends `activate(token, surface)` for the existing
//! surface.
//!
//! This is deliberately narrow: no new toplevel window is created, no token is
//! invented, and the compositor remains free to reject the token. A successful
//! `activate` request only means that the request reached the compositor.

use std::ffi::c_void;
use std::ptr::NonNull;

use wayland_client::backend::{Backend, ObjectId};
use wayland_client::protocol::wl_registry::{Event as RegistryEvent, WlRegistry};
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols::xdg::activation::v1::client::xdg_activation_v1::XdgActivationV1;

/// Raw Wayland display/surface pointers obtained from eframe's creation
/// context. They are only used while the eframe window is alive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WaylandSurface {
    pub display: NonNull<c_void>,
    pub surface: NonNull<c_void>,
}

struct ActivationState {
    globals: Vec<(u32, String, u32)>,
}

impl Dispatch<WlRegistry, ()> for ActivationState {
    fn event(
        state: &mut Self,
        _proxy: &WlRegistry,
        event: RegistryEvent,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        if let RegistryEvent::Global {
            name,
            interface,
            version,
        } = event
        {
            state.globals.push((name, interface, version));
        }
    }
}

impl Dispatch<XdgActivationV1, ()> for ActivationState {
    fn event(
        _state: &mut Self,
        _proxy: &XdgActivationV1,
        _event: <XdgActivationV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
    }
}

/// Apply an activation token to the existing winit surface.
///
/// The caller must pass a token that was created by the compositor for a user
/// action. Tokens are single-use and short-lived; callers should forward them
/// immediately. A returned `Ok(())` means the request was flushed to Wayland,
/// not that the compositor granted focus.
pub fn activate(surface: WaylandSurface, token: &str) -> Result<(), String> {
    if token.trim().is_empty() {
        return Err("leerer Activation-Token".to_owned());
    }

    // Safety: the display pointer comes from eframe/winit and remains valid for
    // the lifetime of the window. This is a guest connection and does not own
    // or close the display.
    let backend = unsafe { Backend::from_foreign_display(surface.display.as_ptr().cast()) };
    let conn = Connection::from_backend(backend);
    let mut queue = conn.new_event_queue();
    let qh = queue.handle();

    let registry = conn.display().get_registry(&qh, ());
    let mut state = ActivationState {
        globals: Vec::new(),
    };
    queue
        .roundtrip(&mut state)
        .map_err(|err| format!("Wayland-Registry-Roundtrip fehlgeschlagen: {err}"))?;

    let Some((name, _interface, _version)) = state
        .globals
        .iter()
        .find(|(_, interface, _)| interface == "xdg_activation_v1")
        .cloned()
    else {
        return Err("Compositor stellt xdg_activation_v1 nicht bereit".to_owned());
    };

    let activation: XdgActivationV1 =
        registry.bind::<XdgActivationV1, (), ActivationState>(name, 1, &qh, ());

    // Safety: the surface pointer belongs to the live winit window and the
    // guest connection shares the same wl_display. The object is only borrowed
    // for this request.
    let surface_id =
        unsafe { ObjectId::from_ptr(WlSurface::interface(), surface.surface.as_ptr().cast()) }
            .map_err(|_| {
                "Wayland-Oberfläche konnte nicht als wl_surface erkannt werden".to_owned()
            })?;
    let wl_surface = WlSurface::from_id(&conn, surface_id)
        .map_err(|_| "Wayland-Oberfläche ist nicht mehr gültig".to_owned())?;

    activation.activate(token.to_owned(), &wl_surface);
    conn.flush()
        .map_err(|err| format!("Activation-Anforderung konnte nicht geflushst werden: {err}"))?;

    // Keep the queue alive until the request has been flushed; the activation
    // object has no reply event.
    drop(queue);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_token_is_rejected_without_touching_wayland() {
        let surface = WaylandSurface {
            display: NonNull::dangling(),
            surface: NonNull::dangling(),
        };
        assert!(activate(surface, "").is_err());
        assert!(activate(surface, "   ").is_err());
    }
}

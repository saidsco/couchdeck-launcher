//! Administrator views.
//!
//! Standard egui widgets are used here so the gamepad can drive them by
//! injecting arrow/Enter key events (see `app.rs`). Every essential admin action
//! is reachable with D-pad + A; text fields remain a keyboard/mouse comfort
//! feature.

use eframe::egui::{
    self, Align, Color32, Id, Rect, RichText, ScrollArea, Sense, Slider, Stroke, TextEdit,
    UiBuilder, Vec2,
};

use crate::app::{AdminView, CouchDeckApp, Overlay, PickerPurpose};
use crate::artwork::ArtworkKind;
use crate::desktop::{self, Severity};
use crate::model::{GameEntry, ProfileRole};
use crate::ui::{
    self, danger_button, paint_texture, primary_button, secondary_button, truncate, ACCENT, CARD,
    DANGER, MUTED, OK, TEXT, WARNING,
};

pub fn draw(ui: &mut egui::Ui, app: &mut CouchDeckApp) {
    let ctx = ui.ctx().clone();
    let scale = ui::ui_scale(&ctx);
    let screen = ctx.screen_rect();
    ui::paint_background(app, ui);

    let margin = 40.0 * scale;
    let header_rect = Rect::from_min_max(
        egui::pos2(screen.left() + margin, screen.top() + margin * 0.65),
        egui::pos2(
            screen.right() - margin,
            screen.top() + margin * 0.65 + 68.0 * scale,
        ),
    );
    ui::header(ui, app, header_rect, scale);

    if !app.admin_unlocked {
        ui.painter().text(
            screen.center(),
            egui::Align2::CENTER_CENTER,
            "Admin-Bereich gesperrt",
            egui::FontId::proportional(28.0 * scale),
            MUTED,
        );
        return;
    }

    let content = Rect::from_min_max(
        egui::pos2(screen.left() + margin, header_rect.bottom() + 20.0 * scale),
        egui::pos2(
            screen.right() - margin,
            screen.bottom() - margin * 0.65 - 58.0 * scale,
        ),
    );
    let footer = Rect::from_min_max(
        egui::pos2(
            screen.left() + margin,
            screen.bottom() - margin * 0.65 - 46.0 * scale,
        ),
        egui::pos2(screen.right() - margin, screen.bottom() - margin * 0.65),
    );

    match app.admin.view.clone() {
        AdminView::Menu => draw_menu(ui, app, content, scale),
        AdminView::Games => draw_games(ui, app, content, scale),
        AdminView::GameEdit(id) => draw_game_edit(ui, app, content, scale, &id),
        AdminView::Profiles => draw_profiles(ui, app, content, scale),
        AdminView::Integrations => draw_integrations(ui, app, content, scale),
        AdminView::Settings => draw_settings(ui, app, content, scale),
        AdminView::Pin => draw_pin(ui, app, content, scale),
    }

    if matches!(app.overlay, Overlay::None) {
        ui::draw_focus_ring(ui, scale);
    }
    draw_footer(ui, app, footer, scale);
}

fn draw_footer(ui: &mut egui::Ui, app: &mut CouchDeckApp, rect: Rect, scale: f32) {
    ui.scope_builder(
        UiBuilder::new().max_rect(rect).id_salt("admin-footer"),
        |ui| {
            ui.horizontal_centered(|ui| {
                ui.label(
                    RichText::new("B zurück   ·   D-Pad/Enter navigieren   ·   M Menü   ·   Maus/Tastatur parallel")
                        .size(13.0 * scale)
                        .color(MUTED),
                );
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    let active = app
                        .active_profile()
                        .map(|p| p.name.clone())
                        .unwrap_or_default();
                    let pin = app
                        .active_profile_id()
                        .map(|id| app.secrets.has_pin(id))
                        .unwrap_or(false);
                    ui.label(
                        RichText::new(format!(
                            "Admin: {active}   ·   PIN: {}",
                            if pin { "gesetzt" } else { "nicht gesetzt" }
                        ))
                        .size(13.0 * scale)
                        .color(if pin { OK } else { WARNING }),
                    );
                });
            });
        },
    );
}

#[derive(Default)]
struct FocusOrder {
    ids: Vec<Id>,
}

impl FocusOrder {
    fn push(&mut self, response: &egui::Response) {
        if response.enabled() && response.sense.interactive() && !self.ids.contains(&response.id) {
            self.ids.push(response.id);
        }
    }
}

fn note_focus(response: &egui::Response, focus: &mut FocusOrder) {
    focus.push(response);
}

/// Apply controller D-pad steps after all widgets of the current admin view
/// have been created. Requesting focus here takes effect on the next frame,
/// while confirm is injected at the beginning of the next frame.
fn finish_focus(ctx: &egui::Context, app: &mut CouchDeckApp, focus: &FocusOrder) {
    if !matches!(app.overlay, Overlay::None) || focus.ids.is_empty() {
        // A pending confirm without a focusable target must not fire later in
        // an unrelated view.
        app.admin.pending_steps = 0;
        app.admin.pending_confirm = false;
        return;
    }
    let steps = std::mem::take(&mut app.admin.pending_steps);
    let current = ctx.memory(|memory| memory.focused());
    let current_index =
        current.and_then(|id| focus.ids.iter().position(|candidate| *candidate == id));
    let target_index = focus_target_index(current_index, focus.ids.len(), steps);
    let target = focus.ids[target_index];
    if current != Some(target) || steps != 0 {
        ctx.memory_mut(|memory| memory.request_focus(target));
        if steps != 0 || current.is_none() {
            if let Some(response) = ctx.read_response(target) {
                response.scroll_to_me(Some(Align::Center));
            }
        }
        ctx.request_repaint();
    }
}

fn focus_target_index(current: Option<usize>, len: usize, steps: i32) -> usize {
    debug_assert!(len > 0);
    match current {
        Some(index) => ((index as isize + steps as isize).rem_euclid(len as isize)) as usize,
        None => {
            if steps < 0 {
                len - 1
            } else {
                0
            }
        }
    }
}

fn heading(ui: &mut egui::Ui, title: &str, subtitle: &str, scale: f32) {
    ui.label(RichText::new(title).size(30.0 * scale).strong().color(TEXT));
    if !subtitle.is_empty() {
        ui.label(RichText::new(subtitle).size(13.5 * scale).color(MUTED));
    }
    ui.add_space(8.0 * scale);
}

fn draw_menu(ui: &mut egui::Ui, app: &mut CouchDeckApp, rect: Rect, scale: f32) {
    let mut focus = FocusOrder::default();
    ui.scope_builder(
        UiBuilder::new().max_rect(rect).id_salt("admin-menu"),
        |ui| {
            heading(
                ui,
                "Admin-Bereich",
                "Nur sichtbar für Admin-Profile. Die PIN ist eine lokale UI-Sperre, keine OS-Sicherheitsgrenze.",
                scale,
            );
            ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                ui.set_max_width(760.0 * scale);
                let entries: [(String, AdminView); 5] = [
                    (
                        format!("Spiele verwalten  ({})", app.library.games.len()),
                        AdminView::Games,
                    ),
                    (
                        format!("Profile verwalten  ({})", app.library.profiles.len()),
                        AdminView::Profiles,
                    ),
                    ("Steam / Prism".to_owned(), AdminView::Integrations),
                    ("Einstellungen".to_owned(), AdminView::Settings),
                    ("PIN & Sicherheit".to_owned(), AdminView::Pin),
                ];
                for (label, view) in entries {
                    let response = ui.add_sized(
                        Vec2::new(760.0 * scale, 56.0 * scale),
                        secondary_button(label),
                    );
                    note_focus(&response, &mut focus);
                    if response.clicked() {
                        enter_view(app, view);
                    }
                }
                ui.add_space(12.0 * scale);
                if !app
                    .active_profile_id()
                    .map(|id| app.secrets.has_pin(id))
                    .unwrap_or(false)
                {
                    ui.label(
                        RichText::new(
                            "Keine Admin-PIN gesetzt. Jeder mit Zugriff auf dieses Linux-Konto kann Änderungen vornehmen.",
                        )
                        .size(13.5 * scale)
                        .color(WARNING),
                    );
                    let response = ui.add_sized(
                        Vec2::new(280.0 * scale, 44.0 * scale),
                        primary_button("PIN jetzt setzen"),
                    );
                    note_focus(&response, &mut focus);
                    if response.clicked() {
                        app.open_set_pin();
                    }
                }
                ui.add_space(12.0 * scale);
                let response = ui.add_sized(
                    Vec2::new(320.0 * scale, 48.0 * scale),
                    secondary_button("Zurück zur Bibliothek  (B)"),
                );
                note_focus(&response, &mut focus);
                if response.clicked() {
                    app.leave_admin();
                }
            });
        },
    );
    finish_focus(ui.ctx(), app, &focus);
}

fn enter_view(app: &mut CouchDeckApp, view: AdminView) {
    app.admin.view = view.clone();
    app.admin.needs_initial_focus = true;
    app.admin.pending_steps = 0;
    app.admin.pending_confirm = false;
    match view {
        AdminView::Games => {
            if app
                .admin
                .selected_game
                .as_ref()
                .map(|id| app.library.game(id).is_none())
                .unwrap_or(true)
            {
                app.admin.selected_game = app.library.games.first().map(|g| g.id.clone());
            }
        }
        AdminView::Profiles => {
            if app
                .admin
                .selected_profile
                .as_ref()
                .map(|id| !app.library.profiles.iter().any(|p| &p.id == id))
                .unwrap_or(true)
            {
                app.admin.selected_profile = app
                    .active_profile_id()
                    .map(ToOwned::to_owned)
                    .or_else(|| app.library.profiles.first().map(|p| p.id.clone()));
            }
        }
        AdminView::Integrations => {
            app.admin.selected_steam_game = None;
            app.admin.selected_instance = None;
            app.admin.prism_title.clear();
        }
        _ => {}
    }
}

fn draw_games(ui: &mut egui::Ui, app: &mut CouchDeckApp, rect: Rect, scale: f32) {
    let mut focus = FocusOrder::default();
    let top_h = 50.0 * scale;
    ui.scope_builder(
        UiBuilder::new().max_rect(rect).id_salt("admin-games"),
        |ui| {
            ui.horizontal(|ui| {
                let back = ui.add_sized(
                    Vec2::new(170.0 * scale, 40.0 * scale),
                    secondary_button("Zurück  (B)"),
                );
                if back.clicked() {
                    app.admin_back();
                    return;
                }
                let import = ui.add_sized(
                    Vec2::new(250.0 * scale, 40.0 * scale),
                    primary_button("Desktop-Datei importieren"),
                );
                note_focus(&import, &mut focus);
                if import.clicked() {
                    app.open_import();
                }
                ui.label(
                    RichText::new(format!(
                        "{} Einträge (Originaldateien werden nie verändert)",
                        app.library.games.len()
                    ))
                    .size(13.5 * scale)
                    .color(MUTED),
                );
            });
            ui.add_space(10.0 * scale);
            let body = Rect::from_min_max(
                egui::pos2(rect.left(), rect.top() + top_h),
                rect.max,
            );
            let left_w = (body.width() * 0.31).clamp(260.0 * scale, 420.0 * scale);
            let left = Rect::from_min_max(body.min, egui::pos2(body.left() + left_w, body.bottom()));
            let right = Rect::from_min_max(
                egui::pos2(left.right() + 22.0 * scale, body.top()),
                body.max,
            );

            let games = app.library.games.clone();
            if app
                .admin
                .selected_game
                .as_ref()
                .map(|id| games.iter().all(|g| &g.id != id))
                .unwrap_or(true)
            {
                app.admin.selected_game = games.first().map(|g| g.id.clone());
            }

            ui.scope_builder(
                UiBuilder::new().max_rect(left).id_salt("admin-game-list"),
                |ui| {
                    ui.label(RichText::new("Spiele").size(18.0 * scale).strong());
                    ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                        if games.is_empty() {
                            ui.label(
                                RichText::new("Noch keine Einträge importiert.")
                                    .size(14.0 * scale)
                                    .color(MUTED),
                            );
                        }
                        for game in &games {
                            let selected = app.admin.selected_game.as_deref() == Some(&game.id);
                            let mut suffix = String::new();
                            if game.hidden {
                                suffix.push_str("  [ausgeblendet]");
                            }
                            if !game.desktop_exists() {
                                suffix.push_str("  [Datei fehlt]");
                            }
                            let label = truncate(&format!("{}{}", game.title(), suffix), 42);
                            let response = ui.add_sized(
                                Vec2::new(ui.available_width(), 40.0 * scale),
                                egui::Button::selectable(selected, label),
                            );
                            note_focus(&response, &mut focus);
                            if response.has_focus() {
                                response.scroll_to_me(Some(Align::Center));
                            }
                            if response.clicked() {
                                app.admin.selected_game = Some(game.id.clone());
                            }
                        }
                    });
                },
            );

            ui.scope_builder(
                UiBuilder::new().max_rect(right).id_salt("admin-game-editor"),
                |ui| {
                    ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                        if let Some(game) = app
                            .admin
                            .selected_game
                            .as_ref()
                            .and_then(|id| app.library.game(id))
                            .cloned()
                        {
                            draw_game_editor(ui, app, &game, scale, &mut focus);
                        } else {
                            ui.label(
                                RichText::new("Wähle links einen Eintrag oder importiere eine Desktop-Datei.")
                                    .size(15.0 * scale)
                                    .color(MUTED),
                            );
                        }
                    });
                },
            );
        },
    );
    finish_focus(ui.ctx(), app, &focus);
}

fn draw_game_editor(
    ui: &mut egui::Ui,
    app: &mut CouchDeckApp,
    game: &GameEntry,
    scale: f32,
    focus: &mut FocusOrder,
) {
    ui.horizontal(|ui| {
        let cover_path = app.game_cover_path(game);
        let preview_size = Vec2::new(92.0 * scale, 123.0 * scale);
        let (rect, _) = ui.allocate_exact_size(preview_size, Sense::hover());
        if let Some(path) = cover_path {
            if let Some(texture) =
                app.artwork
                    .texture(ui.ctx(), &app.paths, &path, ArtworkKind::Cover)
            {
                paint_texture(ui, &texture, rect, 8);
            }
        } else {
            ui.painter()
                .rect_filled(rect, 8, Color32::from_rgb(20, 24, 33));
        }
        ui.vertical(|ui| {
            ui.label(
                RichText::new(truncate(&game.title(), 42))
                    .size(25.0 * scale)
                    .strong()
                    .color(TEXT),
            );
            ui.label(
                RichText::new(if game.managed {
                    "CouchDeck-verwaltete Desktop-Datei"
                } else {
                    "Original-Desktop-Datei (unverändert)"
                })
                .size(12.5 * scale)
                .color(if game.managed { MUTED } else { OK }),
            );
            ui.label(
                RichText::new(truncate(&game.desktop_file.to_string_lossy(), 96))
                    .monospace()
                    .size(11.5 * scale)
                    .color(MUTED),
            );
            if !game.desktop_exists() {
                ui.label(
                    RichText::new("Datei fehlt – Start nicht möglich")
                        .size(13.0 * scale)
                        .color(DANGER),
                );
            }
        });
    });
    ui.add_space(10.0 * scale);

    ui.label(
        RichText::new("Titel-Override")
            .size(13.0 * scale)
            .color(MUTED),
    );
    let mut title = game.title_override.clone().unwrap_or_default();
    let title_response = ui.add(
        TextEdit::singleline(&mut title)
            .desired_width(f32::INFINITY)
            .hint_text("Leer = Name aus Desktop-Datei"),
    );
    note_focus(&title_response, focus);
    if title_response.changed() {
        if let Some(entry) = app.library.game_mut(&game.id) {
            entry.title_override = (!title.trim().is_empty()).then(|| title.trim().to_owned());
        }
    }
    if title_response.lost_focus() {
        app.save_library();
    }

    ui.add_space(6.0 * scale);
    ui.horizontal_wrapped(|ui| {
        let cover = ui.add_sized(
            Vec2::new(180.0 * scale, 40.0 * scale),
            secondary_button("Cover wählen"),
        );
        note_focus(&cover, focus);
        if cover.clicked() {
            app.open_picker(PickerPurpose::Cover(game.id.clone()));
        }
        if game.cover.is_some() {
            let clear = ui.add_sized(
                Vec2::new(160.0 * scale, 40.0 * scale),
                secondary_button("Cover entfernen"),
            );
            note_focus(&clear, focus);
            if clear.clicked() {
                app.clear_game_cover(&game.id);
            }
        }
        let background = ui.add_sized(
            Vec2::new(220.0 * scale, 40.0 * scale),
            secondary_button("Hintergrund wählen"),
        );
        note_focus(&background, focus);
        if background.clicked() {
            app.open_picker(PickerPurpose::Background(game.id.clone()));
        }
        if game.background.is_some() {
            let clear = ui.add_sized(
                Vec2::new(190.0 * scale, 40.0 * scale),
                secondary_button("Hintergrund entfernen"),
            );
            note_focus(&clear, focus);
            if clear.clicked() {
                app.clear_game_background(&game.id);
            }
        }
    });

    ui.horizontal_wrapped(|ui| {
        let hidden_label = if game.hidden {
            "Einblenden"
        } else {
            "Ausblenden"
        };
        let hide = ui.add_sized(
            Vec2::new(170.0 * scale, 40.0 * scale),
            secondary_button(hidden_label),
        );
        note_focus(&hide, focus);
        if hide.clicked() {
            app.toggle_game_hidden(&game.id);
        }
        let up = ui.add_sized(
            Vec2::new(130.0 * scale, 40.0 * scale),
            secondary_button("Nach oben"),
        );
        note_focus(&up, focus);
        if up.clicked() {
            app.move_game(&game.id, true);
        }
        let down = ui.add_sized(
            Vec2::new(130.0 * scale, 40.0 * scale),
            secondary_button("Nach unten"),
        );
        note_focus(&down, focus);
        if down.clicked() {
            app.move_game(&game.id, false);
        }
        let refresh = ui.add_sized(
            Vec2::new(220.0 * scale, 40.0 * scale),
            secondary_button("Metadaten neu einlesen"),
        );
        note_focus(&refresh, focus);
        if refresh.clicked() {
            match desktop::parse_desktop_file(&game.desktop_file) {
                Ok(entry) => {
                    if let Some(stored) = app.library.game_mut(&game.id) {
                        stored.desktop_name = Some(entry.name);
                        stored.desktop_icon = entry.icon;
                    }
                    app.save_library();
                    app.notify("Name und Icon aus Desktop-Datei neu eingelesen.");
                }
                Err(err) => app.notify_error(err.to_string()),
            }
        }
        let remove = ui.add_sized(
            Vec2::new(170.0 * scale, 40.0 * scale),
            danger_button("Entfernen"),
        );
        note_focus(&remove, focus);
        if remove.clicked() {
            app.request_remove_game(&game.id);
        }
    });

    ui.add_space(8.0 * scale);
    ui.separator();

    ui.label(
        RichText::new("Sichtbarkeit für Profile")
            .size(13.0 * scale)
            .color(MUTED),
    );
    let profiles = app.library.profiles.clone();
    let mut visible_ids: Vec<String> = match &game.visible_to_profiles {
        None => profiles.iter().map(|p| p.id.clone()).collect(),
        Some(ids) => ids.clone(),
    };
    let mut visibility_changed = false;
    for profile in &profiles {
        let mut checked = visible_ids.contains(&profile.id);
        let response = ui.checkbox(&mut checked, &profile.name);
        note_focus(&response, focus);
        if response.changed() {
            if checked {
                if !visible_ids.contains(&profile.id) {
                    visible_ids.push(profile.id.clone());
                }
            } else {
                visible_ids.retain(|id| id != &profile.id);
            }
            visibility_changed = true;
        }
    }
    if visibility_changed {
        let all = profiles.iter().all(|p| visible_ids.contains(&p.id));
        if let Some(stored) = app.library.game_mut(&game.id) {
            stored.visible_to_profiles = if all { None } else { Some(visible_ids) };
        }
        app.save_library();
    }

    ui.add_space(8.0 * scale);
    ui.separator();
    ui.label(
        RichText::new("Import- und Launch-Prüfung")
            .size(13.0 * scale)
            .color(MUTED),
    );
    match desktop::parse_desktop_file(&game.desktop_file) {
        Ok(entry) => {
            ui.label(
                RichText::new("Exec (unverändert, wird von GIO ausgeführt):")
                    .size(12.0 * scale)
                    .color(MUTED),
            );
            ui.add(
                egui::Label::new(
                    RichText::new(
                        entry
                            .exec
                            .clone()
                            .unwrap_or_else(|| "(kein Exec)".to_owned()),
                    )
                    .monospace()
                    .size(12.0 * scale)
                    .color(TEXT),
                )
                .wrap(),
            );
            if let Some(try_exec) = &entry.try_exec {
                ui.label(
                    RichText::new(format!("TryExec: {try_exec}"))
                        .monospace()
                        .size(11.5 * scale)
                        .color(MUTED),
                );
            }
            if let Some(workdir) = &entry.workdir {
                ui.label(
                    RichText::new(format!("Path: {workdir}"))
                        .monospace()
                        .size(11.5 * scale)
                        .color(MUTED),
                );
            }
            for diagnostic in desktop::analyze_entry(&entry, &app.paths) {
                ui.label(
                    RichText::new(format!(
                        "{} {}",
                        match diagnostic.severity {
                            Severity::Info => "i",
                            Severity::Warning => "!",
                            Severity::Error => "!",
                        },
                        diagnostic.message
                    ))
                    .size(12.5 * scale)
                    .color(match diagnostic.severity {
                        Severity::Info => MUTED,
                        Severity::Warning => WARNING,
                        Severity::Error => DANGER,
                    }),
                );
            }
            if entry
                .exec
                .as_deref()
                .and_then(desktop::detect_steam_shortcut)
                .map(|steam| !steam.flatpak)
                .unwrap_or(false)
            {
                let repair = ui.add_sized(
                    Vec2::new(330.0 * scale, 42.0 * scale),
                    primary_button("Steam-Flatpak-Variante erzeugen ..."),
                );
                note_focus(&repair, focus);
                if repair.clicked() {
                    app.request_steam_repair(&game.id);
                }
            }
        }
        Err(err) => {
            ui.label(
                RichText::new(format!("Desktop-Datei nicht lesbar: {err}"))
                    .size(13.0 * scale)
                    .color(DANGER),
            );
        }
    }
}

fn draw_game_edit(ui: &mut egui::Ui, app: &mut CouchDeckApp, rect: Rect, scale: f32, id: &str) {
    // Compatibility view: show the same editor as the split view.
    let Some(game) = app.library.game(id).cloned() else {
        app.admin.view = AdminView::Games;
        return;
    };
    let mut focus = FocusOrder::default();
    ui.scope_builder(
        UiBuilder::new().max_rect(rect).id_salt("admin-game-edit"),
        |ui| {
            let back = ui.add_sized(
                Vec2::new(190.0 * scale, 40.0 * scale),
                secondary_button("Zurück  (B)"),
            );
            note_focus(&back, &mut focus);
            if back.clicked() {
                app.admin.view = AdminView::Games;
            }
            ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    draw_game_editor(ui, app, &game, scale, &mut focus);
                });
        },
    );
    finish_focus(ui.ctx(), app, &focus);
}

fn draw_profiles(ui: &mut egui::Ui, app: &mut CouchDeckApp, rect: Rect, scale: f32) {
    let mut focus = FocusOrder::default();
    let top_h = 50.0 * scale;
    ui.scope_builder(
        UiBuilder::new().max_rect(rect).id_salt("admin-profiles"),
        |ui| {
            ui.horizontal(|ui| {
                let back = ui.add_sized(
                    Vec2::new(170.0 * scale, 40.0 * scale),
                    secondary_button("Zurück  (B)"),
                );
                if back.clicked() {
                    app.admin_back();
                    return;
                }
                let add = ui.add_sized(
                    Vec2::new(220.0 * scale, 40.0 * scale),
                    primary_button("Profil hinzufügen"),
                );
                note_focus(&add, &mut focus);
                if add.clicked() {
                    app.add_profile();
                }
                ui.label(
                    RichText::new("Lokale Profile im selben Linux-Konto")
                        .size(13.5 * scale)
                        .color(MUTED),
                );
            });
            ui.add_space(10.0 * scale);
            let body = Rect::from_min_max(egui::pos2(rect.left(), rect.top() + top_h), rect.max);
            let left_w = (body.width() * 0.31).clamp(260.0 * scale, 400.0 * scale);
            let left =
                Rect::from_min_max(body.min, egui::pos2(body.left() + left_w, body.bottom()));
            let right = Rect::from_min_max(
                egui::pos2(left.right() + 22.0 * scale, body.top()),
                body.max,
            );

            let profiles = app.library.profiles.clone();
            if app
                .admin
                .selected_profile
                .as_ref()
                .map(|id| profiles.iter().all(|p| &p.id != id))
                .unwrap_or(true)
            {
                app.admin.selected_profile = app
                    .active_profile_id()
                    .map(ToOwned::to_owned)
                    .or_else(|| profiles.first().map(|p| p.id.clone()));
            }

            ui.scope_builder(
                UiBuilder::new()
                    .max_rect(left)
                    .id_salt("admin-profile-list"),
                |ui| {
                    ui.label(RichText::new("Profile").size(18.0 * scale).strong());
                    ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for profile in &profiles {
                                let selected =
                                    app.admin.selected_profile.as_deref() == Some(&profile.id);
                                let label = format!(
                                    "{}{}",
                                    profile.name,
                                    if profile.is_admin() { "  [Admin]" } else { "" }
                                );
                                let response = ui.add_sized(
                                    Vec2::new(ui.available_width(), 40.0 * scale),
                                    egui::Button::selectable(selected, truncate(&label, 36)),
                                );
                                note_focus(&response, &mut focus);
                                if response.has_focus() {
                                    response.scroll_to_me(Some(Align::Center));
                                }
                                if response.clicked() {
                                    app.admin.selected_profile = Some(profile.id.clone());
                                }
                            }
                        });
                },
            );

            ui.scope_builder(
                UiBuilder::new()
                    .max_rect(right)
                    .id_salt("admin-profile-editor"),
                |ui| {
                    ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            if let Some(profile) = app
                                .admin
                                .selected_profile
                                .as_ref()
                                .and_then(|id| app.library.profiles.iter().find(|p| &p.id == id))
                                .cloned()
                            {
                                draw_profile_editor(ui, app, &profile, scale, &mut focus);
                            } else {
                                ui.label(RichText::new("Kein Profil ausgewählt.").color(MUTED));
                            }
                        });
                },
            );
        },
    );
    finish_focus(ui.ctx(), app, &focus);
}

fn draw_profile_editor(
    ui: &mut egui::Ui,
    app: &mut CouchDeckApp,
    profile: &crate::model::Profile,
    scale: f32,
    focus: &mut FocusOrder,
) {
    ui.horizontal(|ui| {
        let avatar_side = 96.0 * scale;
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(avatar_side), Sense::hover());
        let avatar_path = app.profile_avatar_path(profile);
        if let Some(path) = avatar_path {
            if let Some(texture) =
                app.artwork
                    .texture(ui.ctx(), &app.paths, &path, ArtworkKind::Avatar)
            {
                paint_texture(ui, &texture, rect, (avatar_side * 0.5) as u8);
            }
        }
        ui.vertical(|ui| {
            ui.label(
                RichText::new(truncate(&profile.name, 36))
                    .size(24.0 * scale)
                    .strong(),
            );
            ui.label(
                RichText::new(profile.role.label())
                    .size(13.0 * scale)
                    .color(if profile.is_admin() { ACCENT } else { MUTED }),
            );
            if app.active_profile_id() == Some(profile.id.as_str()) {
                ui.label(RichText::new("Aktives Profil").size(13.0 * scale).color(OK));
            }
        });
    });
    ui.add_space(8.0 * scale);

    ui.label(RichText::new("Anzeigename").size(13.0 * scale).color(MUTED));
    let mut name = profile.name.clone();
    let name_response = ui.add(
        TextEdit::singleline(&mut name)
            .desired_width(f32::INFINITY)
            .hint_text("Name"),
    );
    note_focus(&name_response, focus);
    if name_response.changed() {
        if let Some(stored) = app.library.profiles.iter_mut().find(|p| p.id == profile.id) {
            stored.name = name.clone();
        }
    }
    if name_response.lost_focus() {
        if name.trim().is_empty() {
            if let Some(stored) = app.library.profiles.iter_mut().find(|p| p.id == profile.id) {
                stored.name = "Profil".to_owned();
            }
        }
        app.save_library();
    }

    let admin_count = app.library.profiles.iter().filter(|p| p.is_admin()).count();
    let mut is_admin = profile.is_admin();
    let can_change_role = !(is_admin && admin_count <= 1);
    let role_response = ui.add_enabled(
        can_change_role,
        egui::Checkbox::new(&mut is_admin, "Admin-Rechte für dieses lokale Profil"),
    );
    note_focus(&role_response, focus);
    if role_response.changed() {
        if let Some(stored) = app.library.profiles.iter_mut().find(|p| p.id == profile.id) {
            stored.role = if is_admin {
                ProfileRole::Admin
            } else {
                ProfileRole::Player
            };
        }
        app.save_library();
        // Do not leave the admin UI open for a profile that just lost its
        // admin role.
        if !is_admin && app.active_profile_id() == Some(profile.id.as_str()) {
            app.leave_admin();
            return;
        }
    }

    ui.horizontal_wrapped(|ui| {
        let avatar = ui.add_sized(
            Vec2::new(220.0 * scale, 40.0 * scale),
            secondary_button("Avatar wählen"),
        );
        note_focus(&avatar, focus);
        if avatar.clicked() {
            app.open_picker(PickerPurpose::Avatar(profile.id.clone()));
        }
        if profile.avatar.is_some() {
            let clear = ui.add_sized(
                Vec2::new(180.0 * scale, 40.0 * scale),
                secondary_button("Avatar entfernen"),
            );
            note_focus(&clear, focus);
            if clear.clicked() {
                app.clear_profile_avatar(&profile.id);
            }
        }
        if profile.is_admin() {
            let pin_set = app.secrets.has_pin(&profile.id);
            if app.active_profile_id() == Some(profile.id.as_str()) {
                let pin = ui.add_sized(
                    Vec2::new(if pin_set { 220.0 } else { 200.0 } * scale, 40.0 * scale),
                    secondary_button(if pin_set { "PIN ändern" } else { "PIN setzen" }),
                );
                note_focus(&pin, focus);
                if pin.clicked() {
                    app.open_set_pin();
                }
                if pin_set {
                    let remove = ui.add_sized(
                        Vec2::new(190.0 * scale, 40.0 * scale),
                        danger_button("PIN entfernen"),
                    );
                    note_focus(&remove, focus);
                    if remove.clicked() {
                        app.request_remove_pin();
                    }
                }
            } else {
                ui.label(
                    RichText::new("PIN wird für das aktive Admin-Profil verwaltet.")
                        .size(12.0 * scale)
                        .color(MUTED),
                );
            }
        }
    });

    ui.add_space(12.0 * scale);
    ui.separator();
    let remove = ui.add_sized(
        Vec2::new(230.0 * scale, 42.0 * scale),
        danger_button("Profil entfernen"),
    );
    note_focus(&remove, focus);
    if remove.clicked() {
        app.request_remove_profile(&profile.id);
    }
    if app.active_profile_id() == Some(profile.id.as_str()) {
        ui.label(
            RichText::new(
                "Das aktive Profil kann entfernt werden; danach wird ein anderes Profil aktiv.",
            )
            .size(12.0 * scale)
            .color(MUTED),
        );
    }
}

fn draw_integrations(ui: &mut egui::Ui, app: &mut CouchDeckApp, rect: Rect, scale: f32) {
    let mut focus = FocusOrder::default();
    ui.scope_builder(
        UiBuilder::new().max_rect(rect).id_salt("admin-integrations"),
        |ui| {
            ui.horizontal(|ui| {
                let back = ui.add_sized(
                    Vec2::new(170.0 * scale, 40.0 * scale),
                    secondary_button("Zurück  (B)"),
                );
                if back.clicked() {
                    app.admin_back();
                    return;
                }
                ui.label(
                    RichText::new("GIO startet ausschließlich Dateien – CouchDeck interpretiert Exec nicht.")
                        .size(13.5 * scale)
                        .color(MUTED),
                );
            });
            ui.add_space(10.0 * scale);
            ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                ui.label(RichText::new("Umgebung").size(19.0 * scale).strong());
                ui.horizontal_wrapped(|ui| {
                    status_chip(
                        ui,
                        "GIO",
                        app.gio_message.is_none(),
                        app.gio_message.as_deref().unwrap_or("verfügbar"),
                        scale,
                    );
                    let flatpak = desktop::flatpak_available();
                    status_chip(ui, "Flatpak", flatpak, if flatpak { "verfügbar" } else { "fehlt" }, scale);
                    let steam = desktop::flatpak_app_installed(&app.paths, "com.valvesoftware.Steam");
                    status_chip(ui, "Steam Flatpak", steam, if steam { "installiert" } else { "nicht gefunden" }, scale);
                    let prism = desktop::flatpak_app_installed(&app.paths, "org.prismlauncher.PrismLauncher");
                    status_chip(ui, "Prism Flatpak", prism, if prism { "installiert" } else { "nicht gefunden" }, scale);
                });
                ui.add_space(14.0 * scale);
                ui.separator();
                ui.add_space(8.0 * scale);

                // Steam
                ui.label(RichText::new("Steam-Flatpak-Reparatur").size(19.0 * scale).strong());
                ui.label(
                    RichText::new(
                        "Erkannte Steam-Rungameid-Shortcuts ohne Flatpak-Aufruf. Es wird nur nach ausdrücklicher Bestätigung eine separate verwaltete Datei erzeugt.",
                    )
                    .size(13.0 * scale)
                    .color(MUTED),
                );
                let mut candidates = Vec::new();
                for game in app.library.games.clone() {
                    if let Ok(entry) = desktop::parse_desktop_file(&game.desktop_file) {
                        if let Some(steam) = entry
                            .exec
                            .as_deref()
                            .and_then(desktop::detect_steam_shortcut)
                        {
                            if !steam.flatpak {
                                candidates.push((game, steam));
                            }
                        }
                    }
                }
                if candidates.is_empty() {
                    ui.label(RichText::new("Keine reparaturbedürftigen Steam-Shortcuts gefunden.").size(13.5 * scale).color(MUTED));
                } else {
                    if app
                        .admin
                        .selected_steam_game
                        .as_ref()
                        .map(|id| candidates.iter().all(|(g, _)| &g.id != id))
                        .unwrap_or(true)
                    {
                        app.admin.selected_steam_game = candidates.first().map(|(g, _)| g.id.clone());
                    }
                    for (game, steam) in &candidates {
                        let selected = app.admin.selected_steam_game.as_deref() == Some(&game.id);
                        let label = format!("{}  ·  Steam-App-ID {}", truncate(&game.title(), 40), steam.app_id);
                        let response = ui.add_sized(
                            Vec2::new(ui.available_width().min(900.0 * scale), 40.0 * scale),
                            egui::Button::selectable(selected, label),
                        );
                        note_focus(&response, &mut focus);
                        if response.clicked() {
                            app.admin.selected_steam_game = Some(game.id.clone());
                        }
                    }
                    let steam_installed = desktop::flatpak_app_installed(&app.paths, "com.valvesoftware.Steam");
                    let response = ui.add_enabled(
                        steam_installed,
                        egui::Button::new("Verwaltete Steam-Flatpak-Variante erzeugen ...")
                            .fill(ACCENT)
                            .corner_radius(10),
                    );
                    note_focus(&response, &mut focus);
                    if response.clicked() {
                        if let Some(id) = app.admin.selected_steam_game.clone() {
                            app.request_steam_repair(&id);
                        }
                    }
                    if !steam_installed {
                        ui.label(RichText::new("com.valvesoftware.Steam ist nicht installiert.").size(12.5 * scale).color(WARNING));
                    }
                }
                ui.add_space(14.0 * scale);
                ui.separator();
                ui.add_space(8.0 * scale);

                // Prism
                ui.label(RichText::new("Prism-Launcher-Instanzen").size(19.0 * scale).strong());
                ui.label(
                    RichText::new(
                        "INSTANCE_ID ist der Ordnername der Instanz, nicht der sichtbare Name. Die erzeugte Datei nutzt:\nflatpak run org.prismlauncher.PrismLauncher --launch INSTANCE_ID",
                    )
                    .size(12.5 * scale)
                    .color(MUTED),
                );
                let instances = desktop::list_prism_instances(&app.paths, 100);
                if instances.is_empty() {
                    ui.label(RichText::new("Keine Prism-Instanzen gefunden. Geprüfte Ordner:").size(13.0 * scale).color(WARNING));
                    for path in desktop::prism_instance_dirs(&app.paths) {
                        ui.label(RichText::new(path.display().to_string()).monospace().size(11.5 * scale).color(MUTED));
                    }
                } else {
                    if app
                        .admin
                        .selected_instance
                        .as_ref()
                        .map(|id| instances.iter().all(|(i, _)| i != id))
                        .unwrap_or(true)
                    {
                        app.admin.selected_instance = instances.first().map(|(id, _)| id.clone());
                        if let Some((id, dir)) = instances.first() {
                            app.admin.prism_title = desktop::prism_instance_label(id, dir);
                        }
                    }
                    for (id, dir) in &instances {
                        let selected = app.admin.selected_instance.as_deref() == Some(id);
                        let label = desktop::prism_instance_label(id, dir);
                        let response = ui.add_sized(
                            Vec2::new(ui.available_width().min(900.0 * scale), 38.0 * scale),
                            egui::Button::selectable(selected, truncate(&label, 70)),
                        );
                        note_focus(&response, &mut focus);
                        if response.clicked() {
                            app.admin.selected_instance = Some(id.clone());
                            app.admin.prism_title = label;
                        }
                    }
                    ui.label(RichText::new("Titel für den CouchDeck-Eintrag").size(12.5 * scale).color(MUTED));
                    let title_response = ui.add(
                        TextEdit::singleline(&mut app.admin.prism_title)
                            .desired_width((640.0 * scale).min(ui.available_width())),
                    );
                    note_focus(&title_response, &mut focus);
                    let prism_installed = desktop::flatpak_app_installed(
                        &app.paths,
                        "org.prismlauncher.PrismLauncher",
                    );
                    let response = ui.add_enabled(
                        prism_installed,
                        egui::Button::new("Prism-Eintrag erzeugen")
                            .fill(ACCENT)
                            .corner_radius(10),
                    );
                    note_focus(&response, &mut focus);
                    if response.clicked() {
                        if let Some(id) = app.admin.selected_instance.clone() {
                            let title = app.admin.prism_title.clone();
                            match app.create_prism_entry(&id, &title) {
                                Ok(new_id) => {
                                    app.admin.view = AdminView::Games;
                                    app.admin.selected_game = Some(new_id);
                                }
                                Err(err) => app.message_dialog(
                                    "Prism-Eintrag fehlgeschlagen",
                                    err,
                                    crate::app::MessageSeverity::Error,
                                ),
                            }
                        }
                    }
                    if !prism_installed {
                        ui.label(RichText::new("org.prismlauncher.PrismLauncher ist nicht als Flatpak installiert.").size(12.5 * scale).color(WARNING));
                    }
                }
            });
        },
    );
    finish_focus(ui.ctx(), app, &focus);
}

fn status_chip(ui: &mut egui::Ui, name: &str, ok: bool, detail: &str, scale: f32) {
    let color = if ok { OK } else { DANGER };
    egui::Frame::NONE
        .fill(CARD)
        .stroke(Stroke::new(1.0, color))
        .corner_radius(10)
        .inner_margin(egui::Margin::symmetric(12, 8))
        .show(ui, |ui| {
            ui.label(
                RichText::new(format!("{name}: {detail}"))
                    .size(12.5 * scale)
                    .color(color),
            );
        });
}

fn draw_settings(ui: &mut egui::Ui, app: &mut CouchDeckApp, rect: Rect, scale: f32) {
    let mut focus = FocusOrder::default();
    ui.scope_builder(
        UiBuilder::new().max_rect(rect).id_salt("admin-settings"),
        |ui| {
            ui.horizontal(|ui| {
                let back = ui.add_sized(
                    Vec2::new(170.0 * scale, 40.0 * scale),
                    secondary_button("Zurück  (B)"),
                );
                if back.clicked() {
                    app.admin_back();
                    return;
                }
                ui.label(RichText::new("Einstellungen").size(19.0 * scale).strong());
            });
            ui.add_space(10.0 * scale);
            ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let mut animations = app.settings.animations;
                    let response = ui.checkbox(&mut animations, "Dezente Fokus-Animationen");
                    note_focus(&response, &mut focus);
                    if response.changed() {
                        app.settings.animations = animations;
                        app.save_settings();
                    }
                    let mut backgrounds = app.settings.show_backgrounds;
                    let response = ui.checkbox(&mut backgrounds, "Hintergrundbilder anzeigen");
                    note_focus(&response, &mut focus);
                    if response.changed() {
                        app.settings.show_backgrounds = backgrounds;
                        app.save_settings();
                    }
                    let mut hints = app.settings.show_controller_hints;
                    let response = ui.checkbox(&mut hints, "Controller-Hinweise in der Bibliothek");
                    note_focus(&response, &mut focus);
                    if response.changed() {
                        app.settings.show_controller_hints = hints;
                        app.save_settings();
                    }

                    ui.add_space(10.0 * scale);
                    let mut auto_columns = app.settings.columns.is_none();
                    let response = ui.checkbox(&mut auto_columns, "Automatische Spaltenzahl");
                    note_focus(&response, &mut focus);
                    if response.changed() {
                        app.settings.columns = if auto_columns { None } else { Some(6) };
                        app.save_settings();
                    }
                    if let Some(columns) = app.settings.columns {
                        let mut value = columns as usize;
                        let response = ui.add(Slider::new(&mut value, 3..=6).text("Spalten"));
                        note_focus(&response, &mut focus);
                        if response.changed() {
                            app.settings.columns = Some(value as u8);
                            app.save_settings();
                        }
                    }

                    ui.add_space(12.0 * scale);
                    ui.separator();
                    ui.label(
                        RichText::new("Globales Hintergrundbild")
                            .size(16.0 * scale)
                            .strong(),
                    );
                    let mut has_background = false;
                    if let Some(path) = app.global_background_path() {
                        has_background = true;
                        let (rect, _) = ui.allocate_exact_size(
                            Vec2::new(320.0 * scale, 180.0 * scale),
                            Sense::hover(),
                        );
                        if let Some(texture) = app.artwork.texture(
                            ui.ctx(),
                            &app.paths,
                            &path,
                            ArtworkKind::Background,
                        ) {
                            paint_texture(ui, &texture, rect, 10);
                        }
                    }
                    ui.horizontal_wrapped(|ui| {
                        let choose = ui.add_sized(
                            Vec2::new(260.0 * scale, 40.0 * scale),
                            secondary_button("Hintergrundbild wählen"),
                        );
                        note_focus(&choose, &mut focus);
                        if choose.clicked() {
                            app.open_picker(PickerPurpose::GlobalBackground);
                        }
                        if has_background {
                            let clear = ui.add_sized(
                                Vec2::new(220.0 * scale, 40.0 * scale),
                                secondary_button("Hintergrund entfernen"),
                            );
                            note_focus(&clear, &mut focus);
                            if clear.clicked() {
                                app.clear_global_background();
                            }
                        }
                    });

                    ui.add_space(14.0 * scale);
                    ui.separator();
                    ui.label(RichText::new("Speicherorte").size(16.0 * scale).strong());
                    for (label, path) in [
                        ("Bibliothek", app.paths.library_path()),
                        ("Einstellungen", app.paths.settings_path()),
                        ("Assets", app.paths.assets_dir()),
                        ("Cache", app.paths.cache_dir()),
                        ("Log", app.paths.log_path()),
                    ] {
                        ui.label(
                            RichText::new(format!("{label}: {}", path.display()))
                                .monospace()
                                .size(11.5 * scale)
                                .color(MUTED),
                        );
                    }
                });
        },
    );
    finish_focus(ui.ctx(), app, &focus);
}

fn draw_pin(ui: &mut egui::Ui, app: &mut CouchDeckApp, rect: Rect, scale: f32) {
    let mut focus = FocusOrder::default();
    ui.scope_builder(
        UiBuilder::new().max_rect(rect).id_salt("admin-pin"),
        |ui| {
            ui.horizontal(|ui| {
                let back = ui.add_sized(
                    Vec2::new(170.0 * scale, 40.0 * scale),
                    secondary_button("Zurück  (B)"),
                );
                if back.clicked() {
                    app.admin_back();
                    return;
                }
                ui.label(RichText::new("PIN & Sicherheit").size(19.0 * scale).strong());
            });
            ui.add_space(10.0 * scale);
            ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                let Some(profile_id) = app.active_profile_id().map(ToOwned::to_owned) else {
                    return;
                };
                let has_pin = app.secrets.has_pin(&profile_id);
                ui.label(
                    RichText::new(if has_pin {
                        "Für das aktive Admin-Profil ist eine PIN gesetzt."
                    } else {
                        "Für das aktive Admin-Profil ist keine PIN gesetzt."
                    })
                    .size(16.0 * scale)
                    .color(if has_pin { OK } else { WARNING }),
                );
                ui.label(
                    RichText::new(
                        "Die PIN wird als Argon2id-Hash in einer separaten Datei mit Rechten 0600 gespeichert. Sie schützt vor versehentlichen UI-Änderungen, nicht vor Zugriff über dasselbe Linux-Konto.",
                    )
                    .size(13.0 * scale)
                    .color(MUTED),
                );
                ui.add_space(10.0 * scale);
                let set = ui.add_sized(
                    Vec2::new(260.0 * scale, 46.0 * scale),
                    primary_button(if has_pin { "PIN ändern" } else { "PIN setzen" }),
                );
                note_focus(&set, &mut focus);
                if set.clicked() {
                    app.open_set_pin();
                }
                if has_pin {
                    let remove = ui.add_sized(
                        Vec2::new(240.0 * scale, 46.0 * scale),
                        danger_button("PIN entfernen"),
                    );
                    note_focus(&remove, &mut focus);
                    if remove.clicked() {
                        app.request_remove_pin();
                    }
                }
                ui.add_space(12.0 * scale);
                ui.label(
                    RichText::new(format!("PIN-Datei: {}", app.paths.secrets_path().display()))
                        .monospace()
                        .size(11.5 * scale)
                        .color(MUTED),
                );
                ui.label(
                    RichText::new("Die PIN-Eingabe ist auch mit Controller über das On-Screen-Ziffernfeld möglich.")
                        .size(12.5 * scale)
                        .color(MUTED),
                );
            });
        },
    );
    finish_focus(ui.ctx(), app, &focus);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_steps_wrap_and_start_sensibly() {
        assert_eq!(focus_target_index(None, 5, 1), 0);
        assert_eq!(focus_target_index(None, 5, -1), 4);
        assert_eq!(focus_target_index(Some(0), 5, -1), 4);
        assert_eq!(focus_target_index(Some(4), 5, 1), 0);
        assert_eq!(focus_target_index(Some(2), 5, 0), 2);
    }

    #[test]
    fn injected_enter_activates_the_focused_button() {
        // This mirrors the controller path: focus is requested at the end of
        // one admin frame, then Enter is injected before the next frame's UI.
        let ctx = egui::Context::default();
        let raw = || egui::RawInput {
            screen_rect: Some(Rect::from_min_size(
                egui::Pos2::ZERO,
                Vec2::new(800.0, 600.0),
            )),
            ..Default::default()
        };

        let mut button_id = Id::new("unset");
        ctx.begin_pass(raw());
        egui::CentralPanel::default().show(&ctx, |ui| {
            let response = ui.add(egui::Button::new("Fokus-Test"));
            button_id = response.id;
        });
        ctx.memory_mut(|memory| memory.request_focus(button_id));
        let _ = ctx.end_pass();

        ctx.begin_pass(raw());
        ctx.input_mut(|input| {
            input.events.push(egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            });
        });
        let mut clicked = false;
        egui::CentralPanel::default().show(&ctx, |ui| {
            let response = ui.add(egui::Button::new("Fokus-Test"));
            clicked = response.clicked();
        });
        let _ = ctx.end_pass();
        assert!(
            clicked,
            "injected Enter did not activate the focused button"
        );
    }
}

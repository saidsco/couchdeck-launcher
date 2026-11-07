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
use crate::i18n::LanguagePreference;
use crate::model::{GameEntry, ProfileRole};
use crate::session;
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
            app.t("admin-locked"),
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
        AdminView::Discovery => draw_discovery(ui, app, content, scale),
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
                    RichText::new(app.t("admin-footer-hint"))
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
                    let pin_label = if pin {
                        app.t("admin-pin-status-set")
                    } else {
                        app.t("admin-pin-status-not-set")
                    };
                    ui.label(
                        RichText::new(app.tf(
                            "admin-footer-status",
                            &[("name", active.into()), ("pin", pin_label.into())],
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
                &app.t("admin-menu-title"),
                &app.t("admin-menu-subtitle"),
                scale,
            );
            ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.set_max_width(760.0 * scale);
                    let entries: [(String, AdminView); 6] = [
                        (
                            app.tf(
                                "admin-games-count",
                                &[("count", app.library.games.len().into())],
                            ),
                            AdminView::Games,
                        ),
                        (
                            app.tf(
                                "admin-profiles-count",
                                &[("count", app.library.profiles.len().into())],
                            ),
                            AdminView::Profiles,
                        ),
                        (app.t("admin-discovery"), AdminView::Discovery),
                        (app.t("admin-steam-prism"), AdminView::Integrations),
                        (app.t("admin-settings"), AdminView::Settings),
                        (app.t("admin-pin-security"), AdminView::Pin),
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
                            RichText::new(app.t("admin-no-pin-warning"))
                                .size(13.5 * scale)
                                .color(WARNING),
                        );
                        let response = ui.add_sized(
                            Vec2::new(280.0 * scale, 44.0 * scale),
                            primary_button(app.t("admin-set-pin-now")),
                        );
                        note_focus(&response, &mut focus);
                        if response.clicked() {
                            app.open_set_pin();
                        }
                    }
                    ui.add_space(12.0 * scale);
                    let response = ui.add_sized(
                        Vec2::new(320.0 * scale, 48.0 * scale),
                        secondary_button(app.t("admin-back-library")),
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
        AdminView::Discovery => {
            if app.discovery.is_none() && !app.discovery_loading {
                app.start_discovery();
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
                    secondary_button(app.t("admin-back")),
                );
                if back.clicked() {
                    app.admin_back();
                    return;
                }
                let import = ui.add_sized(
                    Vec2::new(250.0 * scale, 40.0 * scale),
                    primary_button(app.t("admin-import-desktop")),
                );
                note_focus(&import, &mut focus);
                if import.clicked() {
                    app.open_import();
                }
                ui.label(
                    RichText::new(app.tf(
                        "admin-games-count-label",
                        &[("count", app.library.games.len().into())],
                    ))
                    .size(13.5 * scale)
                    .color(MUTED),
                );
            });
            ui.add_space(10.0 * scale);
            let body = Rect::from_min_max(egui::pos2(rect.left(), rect.top() + top_h), rect.max);
            let left_w = (body.width() * 0.31).clamp(260.0 * scale, 420.0 * scale);
            let left =
                Rect::from_min_max(body.min, egui::pos2(body.left() + left_w, body.bottom()));
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
                    ui.label(
                        RichText::new(app.t("admin-games-header"))
                            .size(18.0 * scale)
                            .strong(),
                    );
                    ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            if games.is_empty() {
                                ui.label(
                                    RichText::new(app.t("admin-games-empty"))
                                        .size(14.0 * scale)
                                        .color(MUTED),
                                );
                            }
                            for game in &games {
                                let selected = app.admin.selected_game.as_deref() == Some(&game.id);
                                let mut suffix = String::new();
                                if game.hidden {
                                    suffix.push_str(&format!("  {}", app.t("admin-hidden")));
                                }
                                if !game.desktop_exists() {
                                    suffix.push_str(&format!("  {}", app.t("admin-file-missing")));
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
                UiBuilder::new()
                    .max_rect(right)
                    .id_salt("admin-game-editor"),
                |ui| {
                    ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
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
                                    RichText::new(app.t("admin-select-game"))
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
                    app.t("admin-managed-file")
                } else {
                    app.t("admin-original-file")
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
                    RichText::new(app.t("admin-file-missing-start"))
                        .size(13.0 * scale)
                        .color(DANGER),
                );
            }
        });
    });
    ui.add_space(10.0 * scale);

    ui.label(
        RichText::new(app.t("admin-title-override"))
            .size(13.0 * scale)
            .color(MUTED),
    );
    let mut title = game.title_override.clone().unwrap_or_default();
    let title_response = ui.add(
        TextEdit::singleline(&mut title)
            .desired_width(f32::INFINITY)
            .hint_text(app.t("admin-title-hint")),
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
            secondary_button(app.t("admin-cover-choose")),
        );
        note_focus(&cover, focus);
        if cover.clicked() {
            app.open_picker(PickerPurpose::Cover(game.id.clone()));
        }
        if game.cover.is_some() {
            let clear = ui.add_sized(
                Vec2::new(160.0 * scale, 40.0 * scale),
                secondary_button(app.t("admin-cover-remove")),
            );
            note_focus(&clear, focus);
            if clear.clicked() {
                app.clear_game_cover(&game.id);
            }
        }
        let background = ui.add_sized(
            Vec2::new(220.0 * scale, 40.0 * scale),
            secondary_button(app.t("admin-background-choose")),
        );
        note_focus(&background, focus);
        if background.clicked() {
            app.open_picker(PickerPurpose::Background(game.id.clone()));
        }
        if game.background.is_some() {
            let clear = ui.add_sized(
                Vec2::new(190.0 * scale, 40.0 * scale),
                secondary_button(app.t("admin-background-remove")),
            );
            note_focus(&clear, focus);
            if clear.clicked() {
                app.clear_game_background(&game.id);
            }
        }
    });

    ui.horizontal_wrapped(|ui| {
        let hidden_label = if game.hidden {
            app.t("admin-show")
        } else {
            app.t("admin-hide")
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
            secondary_button(app.t("admin-move-up")),
        );
        note_focus(&up, focus);
        if up.clicked() {
            app.move_game(&game.id, true);
        }
        let down = ui.add_sized(
            Vec2::new(130.0 * scale, 40.0 * scale),
            secondary_button(app.t("admin-move-down")),
        );
        note_focus(&down, focus);
        if down.clicked() {
            app.move_game(&game.id, false);
        }
        let refresh = ui.add_sized(
            Vec2::new(220.0 * scale, 40.0 * scale),
            secondary_button(app.t("admin-refresh")),
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
                    app.notify_key("admin-refresh-toast");
                }
                Err(err) => app.notify_error(err.to_string()),
            }
        }
        let remove = ui.add_sized(
            Vec2::new(170.0 * scale, 40.0 * scale),
            danger_button(app.t("admin-remove")),
        );
        note_focus(&remove, focus);
        if remove.clicked() {
            app.request_remove_game(&game.id);
        }
    });

    ui.add_space(8.0 * scale);
    ui.separator();

    ui.label(
        RichText::new(app.t("admin-visibility"))
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
        RichText::new(app.t("admin-launch-check"))
            .size(13.0 * scale)
            .color(MUTED),
    );
    match desktop::parse_desktop_file(&game.desktop_file) {
        Ok(entry) => {
            ui.label(
                RichText::new(app.t("admin-exec-label"))
                    .size(12.0 * scale)
                    .color(MUTED),
            );
            ui.add(
                egui::Label::new(
                    RichText::new(entry.exec.clone().unwrap_or_else(|| app.t("admin-no-exec")))
                        .monospace()
                        .size(12.0 * scale)
                        .color(TEXT),
                )
                .wrap(),
            );
            if let Some(try_exec) = &entry.try_exec {
                ui.label(
                    RichText::new(app.tf("admin-tryexec", &[("value", try_exec.clone().into())]))
                        .monospace()
                        .size(11.5 * scale)
                        .color(MUTED),
                );
            }
            if let Some(workdir) = &entry.workdir {
                ui.label(
                    RichText::new(app.tf("admin-path", &[("value", workdir.clone().into())]))
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
                    primary_button(app.t("admin-steam-repair-button")),
                );
                note_focus(&repair, focus);
                if repair.clicked() {
                    app.request_steam_repair(&game.id);
                }
            }
        }
        Err(err) => {
            ui.label(
                RichText::new(app.tf(
                    "admin-desktop-unreadable",
                    &[("error", err.to_string().into())],
                ))
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
                secondary_button(app.t("admin-back")),
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
                    secondary_button(app.t("admin-back")),
                );
                if back.clicked() {
                    app.admin_back();
                    return;
                }
                let add = ui.add_sized(
                    Vec2::new(220.0 * scale, 40.0 * scale),
                    primary_button(app.t("admin-add-profile")),
                );
                note_focus(&add, &mut focus);
                if add.clicked() {
                    app.add_profile();
                }
                ui.label(
                    RichText::new(app.t("admin-local-profiles"))
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
                    ui.label(
                        RichText::new(app.t("admin-profiles-header"))
                            .size(18.0 * scale)
                            .strong(),
                    );
                    ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for profile in &profiles {
                                let selected =
                                    app.admin.selected_profile.as_deref() == Some(&profile.id);
                                let mut label = profile.name.clone();
                                if profile.is_admin() {
                                    label.push_str(&format!("  [{}]", app.t("profile-role-admin")));
                                }
                                if profile.is_child {
                                    label.push_str(&format!("  [{}]", app.t("profile-role-child")));
                                }
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
                                ui.label(RichText::new(app.t("admin-profile-none")).color(MUTED));
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
                RichText::new(ui::profile_role_label(app, profile))
                    .size(13.0 * scale)
                    .color(if profile.is_admin() { ACCENT } else { MUTED }),
            );
            if app.active_profile_id() == Some(profile.id.as_str()) {
                ui.label(
                    RichText::new(app.t("admin-active-profile"))
                        .size(13.0 * scale)
                        .color(OK),
                );
            }
        });
    });
    ui.add_space(8.0 * scale);

    ui.label(
        RichText::new(app.t("admin-name-label"))
            .size(13.0 * scale)
            .color(MUTED),
    );
    let mut name = profile.name.clone();
    let name_response = ui.add(
        TextEdit::singleline(&mut name)
            .desired_width(f32::INFINITY)
            .hint_text(app.t("admin-name-hint")),
    );
    note_focus(&name_response, focus);
    if name_response.changed() {
        if let Some(stored) = app.library.profiles.iter_mut().find(|p| p.id == profile.id) {
            stored.name = name.clone();
        }
    }
    if name_response.lost_focus() {
        if name.trim().is_empty() {
            let default_name = app.t("admin-default-profile-name");
            if let Some(stored) = app.library.profiles.iter_mut().find(|p| p.id == profile.id) {
                stored.name = default_name;
            }
        }
        app.save_library();
    }

    let admin_count = app.library.profiles.iter().filter(|p| p.is_admin()).count();
    let mut is_admin = profile.is_admin();
    let can_change_role = !(profile.is_child || is_admin && admin_count <= 1);
    let role_response = ui.add_enabled(
        can_change_role,
        egui::Checkbox::new(&mut is_admin, app.t("admin-role-admin")),
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

    ui.add_space(12.0 * scale);
    ui.separator();
    ui.label(
        RichText::new(app.t("admin-child-budget"))
            .size(16.0 * scale)
            .strong(),
    );
    let mut is_child = profile.is_child;
    let child_response = ui.checkbox(&mut is_child, app.t("admin-child-checkbox"));
    note_focus(&child_response, focus);
    if child_response.changed() {
        if is_child && !app.has_admin_pin_configured() {
            app.message_key(
                "admin-pin-required-title",
                "admin-child-pin-warning",
                crate::app::MessageSeverity::Error,
            );
        } else if let Some(stored) = app.library.profiles.iter_mut().find(|p| p.id == profile.id) {
            stored.is_child = is_child;
            if is_child {
                // A child profile must not expose the admin UI.
                stored.role = ProfileRole::Player;
                if stored.daily_limit_minutes.is_none() {
                    stored.daily_limit_minutes = Some(60);
                }
            }
            app.save_library();
            if is_child && app.active_profile_id() == Some(profile.id.as_str()) {
                app.leave_admin();
                return;
            }
        }
    }

    if is_child {
        let current = app
            .library
            .profiles
            .iter()
            .find(|p| p.id == profile.id)
            .cloned()
            .unwrap_or_else(|| profile.clone());
        ui.horizontal_wrapped(|ui| {
            if let Some(minutes) = current.daily_limit_minutes {
                let minus =
                    ui.add_enabled(minutes > 15, secondary_button(app.t("admin-limit-minus")));
                note_focus(&minus, focus);
                if minus.clicked() {
                    if let Some(stored) =
                        app.library.profiles.iter_mut().find(|p| p.id == profile.id)
                    {
                        stored.daily_limit_minutes = Some(minutes.saturating_sub(15).max(15));
                    }
                    app.save_library();
                }
                ui.label(
                    RichText::new(app.tf("admin-limit", &[("minutes", minutes.into())]))
                        .size(15.0 * scale)
                        .strong()
                        .color(TEXT),
                );
                let plus =
                    ui.add_enabled(minutes < 720, secondary_button(app.t("admin-limit-plus")));
                note_focus(&plus, focus);
                if plus.clicked() {
                    if let Some(stored) =
                        app.library.profiles.iter_mut().find(|p| p.id == profile.id)
                    {
                        stored.daily_limit_minutes = Some((minutes + 15).min(720));
                    }
                    app.save_library();
                }
                let clear = ui.add_sized(
                    Vec2::new(170.0 * scale, 40.0 * scale),
                    secondary_button(app.t("admin-no-limit")),
                );
                note_focus(&clear, focus);
                if clear.clicked() {
                    if let Some(stored) =
                        app.library.profiles.iter_mut().find(|p| p.id == profile.id)
                    {
                        stored.daily_limit_minutes = None;
                    }
                    app.save_library();
                }
            } else {
                let set = ui.add_sized(
                    Vec2::new(220.0 * scale, 40.0 * scale),
                    secondary_button(app.t("admin-set-limit")),
                );
                note_focus(&set, focus);
                if set.clicked() {
                    if let Some(stored) =
                        app.library.profiles.iter_mut().find(|p| p.id == profile.id)
                    {
                        stored.daily_limit_minutes = Some(60);
                    }
                    app.save_library();
                }
            }
        });

        let today = crate::timekeeping::today_string();
        let status = app
            .timekeeping
            .status(&current.id, current.daily_limit_minutes, &today);
        if status.limited {
            ui.label(
                RichText::new(app.tf(
                    "admin-today-usage",
                    &[
                        ("used", (status.used_seconds / 60).into()),
                        ("bonus", status.bonus_minutes.into()),
                        ("remaining", status.remaining_minutes.into()),
                    ],
                ))
                .size(13.0 * scale)
                .color(if status.exhausted { WARNING } else { MUTED }),
            );
            if status.bonus_minutes > 0 {
                let reset = ui.add_sized(
                    Vec2::new(260.0 * scale, 40.0 * scale),
                    secondary_button(app.t("admin-reset-extra")),
                );
                note_focus(&reset, focus);
                if reset.clicked() {
                    app.reset_extra_minutes(&profile.id);
                }
            }
        }
    }

    ui.add_space(12.0 * scale);
    ui.separator();
    ui.horizontal_wrapped(|ui| {
        let avatar = ui.add_sized(
            Vec2::new(220.0 * scale, 40.0 * scale),
            secondary_button(app.t("admin-avatar-choose")),
        );
        note_focus(&avatar, focus);
        if avatar.clicked() {
            app.open_picker(PickerPurpose::Avatar(profile.id.clone()));
        }
        if profile.avatar.is_some() {
            let clear = ui.add_sized(
                Vec2::new(180.0 * scale, 40.0 * scale),
                secondary_button(app.t("admin-avatar-remove")),
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
                    secondary_button(if pin_set {
                        app.t("admin-pin-change")
                    } else {
                        app.t("admin-pin-set")
                    }),
                );
                note_focus(&pin, focus);
                if pin.clicked() {
                    app.open_set_pin();
                }
                if pin_set {
                    let remove = ui.add_sized(
                        Vec2::new(190.0 * scale, 40.0 * scale),
                        danger_button(app.t("admin-pin-remove")),
                    );
                    note_focus(&remove, focus);
                    if remove.clicked() {
                        app.request_remove_pin();
                    }
                }
            } else {
                ui.label(
                    RichText::new(app.t("admin-pin-active-hint"))
                        .size(12.0 * scale)
                        .color(MUTED),
                );
            }
        }
    });

    ui.add_space(12.0 * scale);
    ui.separator();
    ui.label(
        RichText::new(app.t("admin-profile-background"))
            .size(16.0 * scale)
            .strong(),
    );
    if let Some(path) = app.profile_background_path(profile) {
        let (rect, _) =
            ui.allocate_exact_size(Vec2::new(320.0 * scale, 180.0 * scale), Sense::hover());
        if let Some(texture) =
            app.artwork
                .texture(ui.ctx(), &app.paths, &path, ArtworkKind::Background)
        {
            paint_texture(ui, &texture, rect, 10);
        }
    }
    ui.horizontal_wrapped(|ui| {
        let choose = ui.add_sized(
            Vec2::new(260.0 * scale, 40.0 * scale),
            secondary_button(app.t("admin-background-choose")),
        );
        note_focus(&choose, focus);
        if choose.clicked() {
            app.open_picker(PickerPurpose::ProfileBackground(profile.id.clone()));
        }
        if profile.background.is_some() {
            let clear = ui.add_sized(
                Vec2::new(220.0 * scale, 40.0 * scale),
                secondary_button(app.t("admin-background-remove")),
            );
            note_focus(&clear, focus);
            if clear.clicked() {
                app.clear_profile_background(&profile.id);
            }
        }
    });

    ui.add_space(12.0 * scale);
    ui.separator();
    let remove = ui.add_sized(
        Vec2::new(230.0 * scale, 42.0 * scale),
        danger_button(app.t("admin-profile-remove")),
    );
    note_focus(&remove, focus);
    if remove.clicked() {
        app.request_remove_profile(&profile.id);
    }
    if app.active_profile_id() == Some(profile.id.as_str()) {
        ui.label(
            RichText::new(app.t("admin-remove-active-profile-hint"))
                .size(12.0 * scale)
                .color(MUTED),
        );
    }
}

fn draw_integrations(ui: &mut egui::Ui, app: &mut CouchDeckApp, rect: Rect, scale: f32) {
    let mut focus = FocusOrder::default();
    ui.scope_builder(
        UiBuilder::new()
            .max_rect(rect)
            .id_salt("admin-integrations"),
        |ui| {
            ui.horizontal(|ui| {
                let back = ui.add_sized(
                    Vec2::new(170.0 * scale, 40.0 * scale),
                    secondary_button(app.t("admin-back")),
                );
                if back.clicked() {
                    app.admin_back();
                    return;
                }
                ui.label(
                    RichText::new(app.t("admin-integrations-hint"))
                        .size(13.5 * scale)
                        .color(MUTED),
                );
            });
            ui.add_space(10.0 * scale);
            ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(app.t("admin-environment"))
                            .size(19.0 * scale)
                            .strong(),
                    );
                    ui.horizontal_wrapped(|ui| {
                        status_chip(
                            ui,
                            &app.t("admin-status-gio"),
                            app.gio_message.is_none(),
                            &app.gio_message
                                .clone()
                                .unwrap_or_else(|| app.t("admin-gio-available")),
                            scale,
                        );
                        let flatpak = desktop::flatpak_available();
                        status_chip(
                            ui,
                            &app.t("admin-status-flatpak"),
                            flatpak,
                            &if flatpak {
                                app.t("admin-status-available")
                            } else {
                                app.t("admin-status-missing")
                            },
                            scale,
                        );
                        let steam =
                            desktop::flatpak_app_installed(&app.paths, "com.valvesoftware.Steam");
                        status_chip(
                            ui,
                            &app.t("admin-status-steam"),
                            steam,
                            &if steam {
                                app.t("admin-status-installed")
                            } else {
                                app.t("admin-status-not-found")
                            },
                            scale,
                        );
                        let prism = desktop::flatpak_app_installed(
                            &app.paths,
                            "org.prismlauncher.PrismLauncher",
                        );
                        status_chip(
                            ui,
                            &app.t("admin-status-prism"),
                            prism,
                            &if prism {
                                app.t("admin-status-installed")
                            } else {
                                app.t("admin-status-not-found")
                            },
                            scale,
                        );
                    });
                    ui.add_space(14.0 * scale);
                    ui.separator();
                    ui.add_space(8.0 * scale);

                    // Steam
                    ui.label(
                        RichText::new(app.t("admin-steam-repair"))
                            .size(19.0 * scale)
                            .strong(),
                    );
                    ui.label(
                        RichText::new(app.t("admin-steam-repair-hint"))
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
                        ui.label(
                            RichText::new(app.t("admin-steam-none"))
                                .size(13.5 * scale)
                                .color(MUTED),
                        );
                    } else {
                        if app
                            .admin
                            .selected_steam_game
                            .as_ref()
                            .map(|id| candidates.iter().all(|(g, _)| &g.id != id))
                            .unwrap_or(true)
                        {
                            app.admin.selected_steam_game =
                                candidates.first().map(|(g, _)| g.id.clone());
                        }
                        for (game, steam) in &candidates {
                            let selected =
                                app.admin.selected_steam_game.as_deref() == Some(&game.id);
                            let label = app.tf(
                                "admin-steam-candidate",
                                &[
                                    ("title", truncate(&game.title(), 40).into()),
                                    ("id", steam.app_id.clone().into()),
                                ],
                            );
                            let response = ui.add_sized(
                                Vec2::new(ui.available_width().min(900.0 * scale), 40.0 * scale),
                                egui::Button::selectable(selected, label),
                            );
                            note_focus(&response, &mut focus);
                            if response.clicked() {
                                app.admin.selected_steam_game = Some(game.id.clone());
                            }
                        }
                        let steam_installed =
                            desktop::flatpak_app_installed(&app.paths, "com.valvesoftware.Steam");
                        let response = ui.add_enabled(
                            steam_installed,
                            egui::Button::new(app.t("admin-steam-create"))
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
                            ui.label(
                                RichText::new(app.t("admin-steam-not-installed"))
                                    .size(12.5 * scale)
                                    .color(WARNING),
                            );
                        }
                    }
                    ui.add_space(14.0 * scale);
                    ui.separator();
                    ui.add_space(8.0 * scale);

                    // Prism
                    ui.label(
                        RichText::new(app.t("admin-prism-title"))
                            .size(19.0 * scale)
                            .strong(),
                    );
                    ui.label(
                        RichText::new(app.t("admin-integrations-prism-hint"))
                            .size(12.5 * scale)
                            .color(MUTED),
                    );
                    let instances = desktop::list_prism_instances(&app.paths, 100);
                    if instances.is_empty() {
                        ui.label(
                            RichText::new(app.t("admin-prism-none"))
                                .size(13.0 * scale)
                                .color(WARNING),
                        );
                        for path in desktop::prism_instance_dirs(&app.paths) {
                            ui.label(
                                RichText::new(path.display().to_string())
                                    .monospace()
                                    .size(11.5 * scale)
                                    .color(MUTED),
                            );
                        }
                    } else {
                        if app
                            .admin
                            .selected_instance
                            .as_ref()
                            .map(|id| instances.iter().all(|(i, _)| i != id))
                            .unwrap_or(true)
                        {
                            app.admin.selected_instance =
                                instances.first().map(|(id, _)| id.clone());
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
                        ui.label(
                            RichText::new(app.t("admin-prism-entry-title"))
                                .size(12.5 * scale)
                                .color(MUTED),
                        );
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
                            egui::Button::new(app.t("admin-prism-create"))
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
                                        app.t("admin-prism-failed"),
                                        err,
                                        crate::app::MessageSeverity::Error,
                                    ),
                                }
                            }
                        }
                        if !prism_installed {
                            ui.label(
                                RichText::new(app.t("admin-prism-not-installed"))
                                    .size(12.5 * scale)
                                    .color(WARNING),
                            );
                        }
                    }
                });
        },
    );
    finish_focus(ui.ctx(), app, &focus);
}

fn draw_discovery(ui: &mut egui::Ui, app: &mut CouchDeckApp, rect: Rect, scale: f32) {
    let mut focus = FocusOrder::default();
    ui.scope_builder(
        UiBuilder::new().max_rect(rect).id_salt("admin-discovery"),
        |ui| {
            ui.horizontal(|ui| {
                let back = ui.add_sized(
                    Vec2::new(170.0 * scale, 40.0 * scale),
                    secondary_button(app.t("admin-back")),
                );
                note_focus(&back, &mut focus);
                if back.clicked() {
                    app.admin_back();
                    return;
                }
                let rescan = ui.add_sized(
                    Vec2::new(210.0 * scale, 40.0 * scale),
                    secondary_button(app.t("admin-discovery-rescan")),
                );
                note_focus(&rescan, &mut focus);
                if rescan.clicked() {
                    app.rescan_discovery();
                }
                let add = ui.add_sized(
                    Vec2::new(280.0 * scale, 40.0 * scale),
                    primary_button(app.t("admin-discovery-add")),
                );
                note_focus(&add, &mut focus);
                if add.clicked() {
                    app.open_picker(PickerPurpose::SearchDirectory);
                }
                ui.label(
                    RichText::new(app.t("admin-discovery-title"))
                        .size(19.0 * scale)
                        .strong(),
                );
            });
            if app.discovery_loading {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(RichText::new(app.t("admin-discovery-scanning")));
                });
            }
            ui.add_space(8.0 * scale);
            ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let (roots, warnings, entry_count) = app
                        .discovery
                        .as_ref()
                        .map(|discovery| {
                            (
                                discovery.roots.clone(),
                                discovery.warnings.clone(),
                                discovery.entries.len(),
                            )
                        })
                        .unwrap_or_default();
                    ui.label(
                        RichText::new(
                            app.tf("admin-discovery-count", &[("count", entry_count.into())]),
                        )
                        .size(13.0 * scale)
                        .color(MUTED),
                    );

                    ui.add_space(10.0 * scale);
                    ui.label(
                        RichText::new(app.t("admin-discovery-auto"))
                            .size(16.0 * scale)
                            .strong(),
                    );
                    ui.label(
                        RichText::new(app.t("admin-discovery-auto-hint"))
                            .size(11.5 * scale)
                            .color(MUTED),
                    );
                    for root in roots.iter().filter(|root| root.automatic) {
                        let exists = root.path.is_dir();
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(app.t(root.source.label_key()))
                                    .size(12.0 * scale)
                                    .color(ACCENT),
                            );
                            ui.label(
                                RichText::new(root.path.display().to_string())
                                    .monospace()
                                    .size(11.5 * scale)
                                    .color(if exists { MUTED } else { WARNING }),
                            );
                            if !exists {
                                ui.label(
                                    RichText::new(app.t("admin-discovery-missing"))
                                        .size(11.5 * scale)
                                        .color(WARNING),
                                );
                            }
                        });
                    }

                    ui.add_space(12.0 * scale);
                    ui.separator();
                    ui.label(
                        RichText::new(app.t("admin-discovery-custom"))
                            .size(16.0 * scale)
                            .strong(),
                    );
                    let custom: Vec<_> = roots
                        .iter()
                        .filter(|root| !root.automatic)
                        .cloned()
                        .collect();
                    if custom.is_empty() {
                        ui.label(
                            RichText::new(app.t("admin-discovery-custom-empty"))
                                .size(12.5 * scale)
                                .color(MUTED),
                        );
                    }
                    for root in custom {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(root.path.display().to_string())
                                    .monospace()
                                    .size(11.5 * scale)
                                    .color(if root.path.is_dir() { MUTED } else { WARNING }),
                            );
                            if !root.path.is_dir() {
                                ui.label(
                                    RichText::new(app.t("admin-discovery-missing"))
                                        .size(11.5 * scale)
                                        .color(WARNING),
                                );
                            }
                            let remove = ui.add_sized(
                                Vec2::new(150.0 * scale, 34.0 * scale),
                                danger_button(app.t("admin-discovery-remove")),
                            );
                            note_focus(&remove, &mut focus);
                            if remove.clicked() {
                                app.remove_search_dir(&root.path);
                            }
                        });
                    }

                    if !warnings.is_empty() {
                        ui.add_space(12.0 * scale);
                        ui.separator();
                        ui.label(
                            RichText::new(app.t("admin-discovery-warnings"))
                                .size(16.0 * scale)
                                .strong()
                                .color(WARNING),
                        );
                        for warning in warnings.iter().take(20) {
                            ui.label(
                                RichText::new(format!(
                                    "{}: {}",
                                    warning.path.display(),
                                    warning.message
                                ))
                                .monospace()
                                .size(11.0 * scale)
                                .color(WARNING),
                            );
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
                    secondary_button(app.t("admin-back")),
                );
                if back.clicked() {
                    app.admin_back();
                    return;
                }
                ui.label(
                    RichText::new(app.t("admin-settings"))
                        .size(19.0 * scale)
                        .strong(),
                );
            });
            ui.add_space(10.0 * scale);
            ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(app.t("admin-language"))
                            .size(16.0 * scale)
                            .strong(),
                    );
                    ui.horizontal_wrapped(|ui| {
                        for (preference, label) in [
                            (LanguagePreference::Auto, app.t("admin-language-auto")),
                            (LanguagePreference::De, app.t("admin-language-de")),
                            (LanguagePreference::En, app.t("admin-language-en")),
                        ] {
                            let selected = app.settings.language == preference;
                            let response = ui.add_sized(
                                Vec2::new(160.0 * scale, 40.0 * scale),
                                if selected {
                                    primary_button(label)
                                } else {
                                    secondary_button(label)
                                },
                            );
                            note_focus(&response, &mut focus);
                            if response.clicked() {
                                app.set_language(preference);
                            }
                        }
                    });
                    ui.label(
                        RichText::new(app.tf(
                            "admin-language-status",
                            &[("language", app.i18n.language().label().into())],
                        ))
                        .size(11.5 * scale)
                        .color(MUTED),
                    );

                    ui.add_space(12.0 * scale);
                    ui.separator();
                    let mut animations = app.settings.animations;
                    let response = ui.checkbox(&mut animations, app.t("admin-animations"));
                    note_focus(&response, &mut focus);
                    if response.changed() {
                        app.settings.animations = animations;
                        app.save_settings();
                    }
                    let mut backgrounds = app.settings.show_backgrounds;
                    let response = ui.checkbox(&mut backgrounds, app.t("admin-show-backgrounds"));
                    note_focus(&response, &mut focus);
                    if response.changed() {
                        app.settings.show_backgrounds = backgrounds;
                        app.save_settings();
                    }
                    let mut hints = app.settings.show_controller_hints;
                    let response = ui.checkbox(&mut hints, app.t("admin-controller-hints"));
                    note_focus(&response, &mut focus);
                    if response.changed() {
                        app.settings.show_controller_hints = hints;
                        app.save_settings();
                    }
                    let mut alarm_sound = app.settings.alarm_sound;
                    let response = ui.checkbox(&mut alarm_sound, app.t("admin-alarm-sound"));
                    note_focus(&response, &mut focus);
                    if response.changed() {
                        app.settings.alarm_sound = alarm_sound;
                        app.save_settings();
                    }

                    ui.add_space(12.0 * scale);
                    ui.separator();
                    ui.label(
                        RichText::new(app.t("admin-autostart-header"))
                            .size(16.0 * scale)
                            .strong(),
                    );
                    let session_info = session::detect_current();
                    let supported = session_info.is_supported_desktop();
                    let autostart_enabled = session::autostart_enabled(&app.paths);
                    let autostart_file_exists = session::autostart_file_exists(&app.paths);
                    let mut autostart = autostart_enabled;
                    let response = ui.add_enabled(
                        supported,
                        egui::Checkbox::new(&mut autostart, app.t("admin-autostart")),
                    );
                    note_focus(&response, &mut focus);
                    if response.changed() && supported {
                        let result = if autostart {
                            desktop::current_launcher_binary()
                                .map_err(|err| {
                                    app.tf("admin-autostart-error", &[("error", err.into())])
                                })
                                .and_then(|binary| session::write_autostart(&app.paths, &binary))
                                .map(|path| {
                                    app.tf(
                                        "admin-autostart-enabled",
                                        &[("path", path.display().to_string().into())],
                                    )
                                })
                        } else {
                            session::remove_autostart(&app.paths)
                                .map(|_| app.t("admin-autostart-disabled"))
                        };
                        match result {
                            Ok(message) => app.notify(message),
                            Err(err) => {
                                let title = app.t("admin-autostart-change-error");
                                app.message_dialog(title, err, crate::app::MessageSeverity::Error);
                            }
                        }
                    }
                    if !supported {
                        let reason = if session_info.gaming_mode {
                            app.t("admin-autostart-gaming-mode")
                        } else {
                            app.t("admin-autostart-unsupported")
                        };
                        ui.label(RichText::new(reason).size(12.5 * scale).color(WARNING));
                    } else if let Some(environment) = session_info.environment {
                        ui.label(
                            RichText::new(app.tf(
                                "admin-autostart-session",
                                &[("desktop", app.t(environment.label_key()).into())],
                            ))
                            .size(12.0 * scale)
                            .color(MUTED),
                        );
                    }
                    ui.label(
                        RichText::new(
                            app.tf(
                                "admin-autostart-status",
                                &[
                                    (
                                        "status",
                                        if autostart_enabled {
                                            app.t("admin-autostart-active")
                                        } else if autostart_file_exists {
                                            app.t("admin-autostart-file-inactive")
                                        } else {
                                            app.t("admin-autostart-inactive")
                                        }
                                        .into(),
                                    ),
                                    (
                                        "path",
                                        app.paths.autostart_path().display().to_string().into(),
                                    ),
                                ],
                            ),
                        )
                        .monospace()
                        .size(11.5 * scale)
                        .color(MUTED),
                    );
                    if autostart_file_exists && !autostart_enabled {
                        let remove = ui.add_sized(
                            Vec2::new(260.0 * scale, 38.0 * scale),
                            secondary_button(app.t("admin-autostart-remove")),
                        );
                        note_focus(&remove, &mut focus);
                        if remove.clicked() {
                            match session::remove_autostart(&app.paths) {
                                Ok(_) => app.notify_key("admin-autostart-disabled"),
                                Err(err) => {
                                    let title = app.t("admin-autostart-change-error");
                                    app.message_dialog(
                                        title,
                                        err,
                                        crate::app::MessageSeverity::Error,
                                    );
                                }
                            }
                        }
                    }
                    if let Ok(binary) = desktop::current_launcher_binary() {
                        ui.label(
                            RichText::new(app.tf(
                                "admin-autostart-binary",
                                &[("path", binary.display().to_string().into())],
                            ))
                            .monospace()
                            .size(11.5 * scale)
                            .color(
                                if binary.to_string_lossy().contains("/target/") {
                                    WARNING
                                } else {
                                    MUTED
                                },
                            ),
                        );
                        if binary.to_string_lossy().contains("/target/") {
                            ui.label(
                                RichText::new(app.t("admin-autostart-dev-warning"))
                                    .size(12.0 * scale)
                                    .color(WARNING),
                            );
                        }
                    }

                    ui.add_space(10.0 * scale);
                    let mut auto_columns = app.settings.columns.is_none();
                    let response = ui.checkbox(&mut auto_columns, app.t("admin-auto-columns"));
                    note_focus(&response, &mut focus);
                    if response.changed() {
                        app.settings.columns = if auto_columns { None } else { Some(6) };
                        app.save_settings();
                    }
                    if let Some(columns) = app.settings.columns {
                        let mut value = columns as usize;
                        let response =
                            ui.add(Slider::new(&mut value, 3..=6).text(app.t("admin-columns")));
                        note_focus(&response, &mut focus);
                        if response.changed() {
                            app.settings.columns = Some(value as u8);
                            app.save_settings();
                        }
                    }

                    ui.add_space(12.0 * scale);
                    ui.separator();
                    ui.label(
                        RichText::new(app.t("admin-global-background"))
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
                            secondary_button(app.t("admin-background-image-choose")),
                        );
                        note_focus(&choose, &mut focus);
                        if choose.clicked() {
                            app.open_picker(PickerPurpose::GlobalBackground);
                        }
                        if has_background {
                            let clear = ui.add_sized(
                                Vec2::new(220.0 * scale, 40.0 * scale),
                                secondary_button(app.t("admin-background-remove")),
                            );
                            note_focus(&clear, &mut focus);
                            if clear.clicked() {
                                app.clear_global_background();
                            }
                        }
                    });

                    ui.add_space(14.0 * scale);
                    ui.separator();
                    ui.label(
                        RichText::new(app.t("admin-storage"))
                            .size(16.0 * scale)
                            .strong(),
                    );
                    for (label, path) in [
                        (app.t("admin-storage-library"), app.paths.library_path()),
                        (app.t("admin-storage-settings"), app.paths.settings_path()),
                        (app.t("admin-storage-assets"), app.paths.assets_dir()),
                        (app.t("admin-storage-cache"), app.paths.cache_dir()),
                        (app.t("admin-storage-log"), app.paths.log_path()),
                    ] {
                        ui.label(
                            RichText::new(app.tf(
                                "admin-storage-line",
                                &[
                                    ("label", label.into()),
                                    ("path", path.display().to_string().into()),
                                ],
                            ))
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
    ui.scope_builder(UiBuilder::new().max_rect(rect).id_salt("admin-pin"), |ui| {
        ui.horizontal(|ui| {
            let back = ui.add_sized(
                Vec2::new(170.0 * scale, 40.0 * scale),
                secondary_button(app.t("admin-back")),
            );
            if back.clicked() {
                app.admin_back();
                return;
            }
            ui.label(
                RichText::new(app.t("admin-pin-header"))
                    .size(19.0 * scale)
                    .strong(),
            );
        });
        ui.add_space(10.0 * scale);
        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let Some(profile_id) = app.active_profile_id().map(ToOwned::to_owned) else {
                    return;
                };
                let has_pin = app.secrets.has_pin(&profile_id);
                ui.label(
                    RichText::new(if has_pin {
                        app.t("admin-pin-set-status")
                    } else {
                        app.t("admin-pin-unset-status")
                    })
                    .size(16.0 * scale)
                    .color(if has_pin { OK } else { WARNING }),
                );
                ui.label(
                    RichText::new(app.t("admin-pin-storage-hint"))
                        .size(13.0 * scale)
                        .color(MUTED),
                );
                ui.add_space(10.0 * scale);
                let set = ui.add_sized(
                    Vec2::new(260.0 * scale, 46.0 * scale),
                    primary_button(if has_pin {
                        app.t("admin-pin-change")
                    } else {
                        app.t("admin-pin-set")
                    }),
                );
                note_focus(&set, &mut focus);
                if set.clicked() {
                    app.open_set_pin();
                }
                if has_pin {
                    let remove = ui.add_sized(
                        Vec2::new(240.0 * scale, 46.0 * scale),
                        danger_button(app.t("admin-pin-remove")),
                    );
                    note_focus(&remove, &mut focus);
                    if remove.clicked() {
                        app.request_remove_pin();
                    }
                }
                ui.add_space(12.0 * scale);
                ui.label(
                    RichText::new(app.tf(
                        "admin-pin-file",
                        &[(
                            "path",
                            app.paths.secrets_path().display().to_string().into(),
                        )],
                    ))
                    .monospace()
                    .size(11.5 * scale)
                    .color(MUTED),
                );
                ui.label(
                    RichText::new(app.t("admin-pin-controller-hint"))
                        .size(12.5 * scale)
                        .color(MUTED),
                );
            });
    });
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

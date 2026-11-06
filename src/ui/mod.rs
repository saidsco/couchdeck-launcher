//! Shared UI helpers and screen dispatch.

pub mod admin;
pub mod dialogs;
pub mod library;
pub mod profiles;
pub mod time;

use eframe::egui::{
    self, Align, Color32, CornerRadius, FontId, Id, Margin, Rect, RichText, Sense, Stroke,
    TextStyle, UiBuilder, Vec2,
};

use crate::app::{CouchDeckApp, ToastKind};
use crate::model::GameEntry;

pub const ACCENT: Color32 = Color32::from_rgb(255, 118, 64);
pub const ACCENT_SOFT: Color32 = Color32::from_rgb(90, 50, 34);
pub const BACKGROUND: Color32 = Color32::from_rgb(9, 12, 18);
pub const CARD: Color32 = Color32::from_rgb(24, 29, 39);
pub const CARD_HOVER: Color32 = Color32::from_rgb(33, 40, 53);
pub const TEXT: Color32 = Color32::from_rgb(238, 241, 246);
pub const MUTED: Color32 = Color32::from_rgb(154, 164, 180);
pub const DANGER: Color32 = Color32::from_rgb(239, 92, 92);
pub const WARNING: Color32 = Color32::from_rgb(245, 190, 80);
pub const OK: Color32 = Color32::from_rgb(100, 205, 140);

pub fn ui_scale(ctx: &egui::Context) -> f32 {
    (ctx.screen_rect().height() / 1080.0).clamp(0.80, 2.2)
}

pub fn draw(app: &mut CouchDeckApp, ctx: &egui::Context) {
    if app.fatal.is_some() {
        dialogs::draw_fatal(app, ctx);
        return;
    }
    let scale = ui_scale(ctx);
    if (app.style_scale - scale).abs() > 0.01 {
        apply_style(ctx, scale);
        app.style_scale = scale;
    }
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE)
        .show(ctx, |ui| match app.screen {
            crate::app::Screen::Library => library::draw(ui, app),
            crate::app::Screen::Profiles => profiles::draw(ui, app),
            crate::app::Screen::Admin => admin::draw(ui, app),
        });
    dialogs::draw_overlay(app, ctx);
    dialogs::draw_power(app, ctx);
    dialogs::draw_launch(app, ctx);
    draw_toast(app, ctx);
}

fn apply_style(ctx: &egui::Context, scale: f32) {
    let mut style = (*ctx.style()).clone();
    style
        .text_styles
        .insert(TextStyle::Heading, FontId::proportional(34.0 * scale));
    style
        .text_styles
        .insert(TextStyle::Body, FontId::proportional(19.0 * scale));
    style
        .text_styles
        .insert(TextStyle::Button, FontId::proportional(19.0 * scale));
    style
        .text_styles
        .insert(TextStyle::Small, FontId::proportional(14.0 * scale));
    style
        .text_styles
        .insert(TextStyle::Monospace, FontId::monospace(15.0 * scale));
    style.spacing.item_spacing = Vec2::new(10.0 * scale, 10.0 * scale);
    style.spacing.button_padding = Vec2::new(16.0 * scale, 10.0 * scale);
    style.spacing.interact_size = Vec2::new(42.0 * scale, 42.0 * scale);
    style.visuals = egui::Visuals::dark();
    style.visuals.override_text_color = Some(TEXT);
    style.visuals.panel_fill = Color32::TRANSPARENT;
    style.visuals.window_fill = CARD;
    style.visuals.faint_bg_color = CARD_HOVER;
    style.visuals.extreme_bg_color = Color32::from_rgb(13, 16, 23);
    style.visuals.window_corner_radius = CornerRadius::same((18.0 * scale) as u8);
    style.visuals.menu_corner_radius = CornerRadius::same((14.0 * scale) as u8);
    style.visuals.window_stroke = Stroke::new(1.0, Color32::from_rgb(52, 60, 75));
    style.visuals.selection.bg_fill = ACCENT;
    style.visuals.selection.stroke = Stroke::new(2.0, Color32::WHITE);
    style.visuals.widgets.noninteractive.bg_fill = CARD;
    style.visuals.widgets.noninteractive.weak_bg_fill = CARD;
    style.visuals.widgets.noninteractive.bg_stroke = Stroke::NONE;
    style.visuals.widgets.inactive.weak_bg_fill = CARD_HOVER;
    style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(48, 56, 70));
    style.visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(44, 53, 69);
    style.visuals.widgets.hovered.bg_stroke = Stroke::new(2.0, ACCENT);
    style.visuals.widgets.active.weak_bg_fill = ACCENT_SOFT;
    style.visuals.widgets.active.bg_stroke = Stroke::new(2.0, ACCENT);
    style.visuals.widgets.open.weak_bg_fill = CARD_HOVER;
    style.visuals.warn_fg_color = WARNING;
    style.visuals.error_fg_color = DANGER;
    style.visuals.hyperlink_color = ACCENT;
    style.visuals.button_frame = true;
    ctx.set_style(style);
}

pub fn primary_button(text: impl Into<String>) -> egui::Button<'static> {
    egui::Button::new(
        RichText::new(text.into())
            .strong()
            .color(Color32::from_rgb(24, 15, 8)),
    )
    .fill(ACCENT)
    .stroke(Stroke::NONE)
    .corner_radius(12)
}

pub fn secondary_button(text: impl Into<String>) -> egui::Button<'static> {
    egui::Button::new(RichText::new(text.into()).color(TEXT))
        .fill(CARD_HOVER)
        .stroke(Stroke::new(1.0, Color32::from_rgb(62, 72, 90)))
        .corner_radius(12)
}

pub fn danger_button(text: impl Into<String>) -> egui::Button<'static> {
    egui::Button::new(RichText::new(text.into()).strong().color(Color32::WHITE))
        .fill(Color32::from_rgb(160, 48, 48))
        .stroke(Stroke::new(1.0, DANGER))
        .corner_radius(12)
}

pub fn paint_background(app: &mut CouchDeckApp, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();
    let rect = ctx.screen_rect();
    ui.painter().rect_filled(rect, 0, BACKGROUND);

    if app.settings.show_backgrounds {
        let active_profile_background = app.active_profile_background_path();
        let global_background = app.global_background_path();
        let candidates: Vec<std::path::PathBuf> = match app.screen {
            crate::app::Screen::Library => [
                app.focused_game()
                    .and_then(|game| app.game_background_path(&game)),
                active_profile_background,
                global_background,
            ]
            .into_iter()
            .flatten()
            .collect(),
            crate::app::Screen::Profiles => [
                app.library
                    .profiles
                    .get(app.profile_focus)
                    .and_then(|profile| app.profile_background_path(profile)),
                global_background,
            ]
            .into_iter()
            .flatten()
            .collect(),
            crate::app::Screen::Admin => [active_profile_background, global_background]
                .into_iter()
                .flatten()
                .collect(),
        };
        for path in candidates {
            if let Some(texture) = app.artwork.texture(
                &ctx,
                &app.paths,
                &path,
                crate::artwork::ArtworkKind::Background,
            ) {
                ui.painter().image(
                    texture.id(),
                    rect,
                    Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
                break;
            }
        }
    }

    paint_vertical_gradient(
        ui.painter(),
        rect,
        Color32::from_black_alpha(45),
        Color32::from_black_alpha(165),
    );
    ui.painter()
        .rect_filled(rect, 0, Color32::from_rgba_unmultiplied(8, 11, 18, 35));
}

pub fn paint_vertical_gradient(painter: &egui::Painter, rect: Rect, top: Color32, bottom: Color32) {
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(rect.left_top(), top);
    mesh.colored_vertex(rect.right_top(), top);
    mesh.colored_vertex(rect.right_bottom(), bottom);
    mesh.colored_vertex(rect.left_bottom(), bottom);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(egui::Shape::mesh(mesh));
}

pub fn paint_texture(ui: &egui::Ui, texture: &egui::TextureHandle, rect: Rect, corner: u8) {
    egui::Image::new((texture.id(), rect.size()))
        .fit_to_exact_size(rect.size())
        .corner_radius(corner)
        .paint_at(ui, rect);
}

pub fn header(ui: &mut egui::Ui, app: &mut CouchDeckApp, rect: Rect, scale: f32) {
    ui.scope_builder(UiBuilder::new().max_rect(rect).id_salt("header"), |ui| {
        ui.horizontal_centered(|ui| {
            let title = RichText::new("COUCHDECK")
                .size(26.0 * scale)
                .strong()
                .color(TEXT)
                .extra_letter_spacing(2.0);
            if ui
                .add(
                    egui::Button::new(title)
                        .frame(false)
                        .fill(Color32::TRANSPARENT),
                )
                .clicked()
            {
                if app.screen == crate::app::Screen::Admin {
                    app.leave_admin();
                } else {
                    app.screen = crate::app::Screen::Library;
                    app.overlay = crate::app::Overlay::None;
                }
            }
            ui.add_space(8.0 * scale);
            let section = app.t(match app.screen {
                crate::app::Screen::Library => "header-library",
                crate::app::Screen::Profiles => "header-profiles",
                crate::app::Screen::Admin => "header-admin",
            });
            ui.label(
                RichText::new(section)
                    .size(12.0 * scale)
                    .color(MUTED)
                    .extra_letter_spacing(2.0),
            );

            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add(
                        secondary_button(app.t("header-menu"))
                            .min_size(Vec2::new(150.0 * scale, 42.0 * scale)),
                    )
                    .clicked()
                {
                    app.open_menu();
                }
                profile_chip(ui, app, scale);
                if let Some(time_status) = app.active_time_status() {
                    let (text, color) = if !time_status.limited {
                        (app.t("header-time-unlimited"), MUTED)
                    } else if time_status.exhausted {
                        (app.t("header-time-exhausted"), DANGER)
                    } else {
                        (
                            app.tf(
                                "header-time-remaining",
                                &[("minutes", time_status.remaining_minutes.into())],
                            ),
                            if time_status.remaining_seconds <= 300 {
                                WARNING
                            } else {
                                MUTED
                            },
                        )
                    };
                    ui.label(RichText::new(text).size(12.5 * scale).color(color));
                }
                let status = if let Some(err) = &app.controller_error {
                    app.tf(
                        "header-input-keyboard-error",
                        &[("error", err.clone().into())],
                    )
                } else if app.controllers.is_empty() {
                    app.t("header-input-keyboard")
                } else if app.controllers.len() == 1 {
                    app.tf(
                        "header-controller",
                        &[("name", app.controllers[0].clone().into())],
                    )
                } else {
                    app.tf(
                        "header-controllers",
                        &[("count", app.controllers.len().into())],
                    )
                };
                ui.label(
                    RichText::new(truncate(&status, 46))
                        .size(12.5 * scale)
                        .color(MUTED),
                );
            });
        });
    });
}

fn profile_chip(ui: &mut egui::Ui, app: &mut CouchDeckApp, scale: f32) {
    let Some(profile) = app.active_profile().cloned() else {
        return;
    };
    let avatar_path = app.profile_avatar_path(&profile);
    let avatar_tex = avatar_path.and_then(|path| {
        app.artwork.texture(
            ui.ctx(),
            &app.paths,
            &path,
            crate::artwork::ArtworkKind::Avatar,
        )
    });
    let size = Vec2::splat(42.0 * scale);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    if let Some(texture) = avatar_tex {
        paint_texture(ui, &texture, rect, (21.0 * scale) as u8);
    } else {
        ui.painter()
            .circle_filled(rect.center(), rect.width() * 0.5, ACCENT_SOFT);
        let initial = profile
            .name
            .chars()
            .next()
            .map(|c| c.to_uppercase().to_string())
            .unwrap_or_else(|| "?".to_owned());
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            initial,
            FontId::proportional(19.0 * scale),
            TEXT,
        );
    }
    let response = response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(app.t("profile-switch"));
    if response.clicked() {
        app.open_profiles();
    }
    if ui
        .add(
            egui::Button::new(
                RichText::new(format!(
                    "{}  ·  {}",
                    truncate(&profile.name, 22),
                    profile_role_label(app, &profile)
                ))
                .size(15.0 * scale)
                .color(TEXT),
            )
            .frame(false),
        )
        .on_hover_text(app.t("profile-switch"))
        .clicked()
    {
        app.open_profiles();
    }
}

/// Draw a visible focus ring around the currently keyboard-focused widget.
/// Admin views use egui's native focus navigation (fed by controller keys), and
/// explicit button fills otherwise hide the focus state.
pub fn draw_focus_ring(ui: &egui::Ui, scale: f32) {
    let ctx = ui.ctx();
    if let Some(id) = ctx.memory(|memory| memory.focused()) {
        if let Some(response) = ctx.read_response(id) {
            if response.rect.is_positive() {
                ui.painter().rect_stroke(
                    response.rect.expand(3.0 * scale),
                    CornerRadius::same((12.0 * scale) as u8),
                    Stroke::new(2.5 * scale, ACCENT),
                    egui::StrokeKind::Outside,
                );
            }
        }
    }
}

pub fn profile_role_label(app: &CouchDeckApp, profile: &crate::model::Profile) -> String {
    if profile.is_child {
        app.t("profile-role-child")
    } else if profile.is_admin() {
        app.t("profile-role-admin")
    } else {
        app.t("profile-role-player")
    }
}

pub fn truncate(text: &str, max_chars: usize) -> String {
    let count = text.chars().count();
    if count <= max_chars {
        text.to_owned()
    } else {
        let mut out: String = text.chars().take(max_chars.saturating_sub(1)).collect();
        out.push_str("...");
        out
    }
}

pub enum TileImage {
    Cover(egui::TextureHandle),
    Icon(egui::TextureHandle),
}

pub fn game_tile_image(
    app: &mut CouchDeckApp,
    ctx: &egui::Context,
    game: &GameEntry,
) -> Option<TileImage> {
    if let Some(path) = app.game_cover_path(game) {
        if let Some(texture) =
            app.artwork
                .texture(ctx, &app.paths, &path, crate::artwork::ArtworkKind::Cover)
        {
            return Some(TileImage::Cover(texture));
        }
    }
    let icon_name = game.desktop_icon.as_deref()?;
    let path = app.icons.resolve(icon_name)?;
    app.artwork
        .texture(ctx, &app.paths, &path, crate::artwork::ArtworkKind::Icon)
        .map(TileImage::Icon)
}

fn draw_toast(app: &CouchDeckApp, ctx: &egui::Context) {
    let Some(toast) = &app.toast else {
        return;
    };
    let scale = ui_scale(ctx);
    let color = match toast.kind {
        ToastKind::Info => Color32::from_rgb(30, 38, 50),
        ToastKind::Warning => Color32::from_rgb(78, 58, 24),
        ToastKind::Error => Color32::from_rgb(86, 30, 30),
    };
    let stroke = match toast.kind {
        ToastKind::Info => Stroke::new(1.0, Color32::from_rgb(62, 75, 94)),
        ToastKind::Warning => Stroke::new(1.0, WARNING),
        ToastKind::Error => Stroke::new(1.0, DANGER),
    };
    egui::Area::new(Id::new("toast"))
        .anchor(egui::Align2::CENTER_BOTTOM, Vec2::new(0.0, -26.0 * scale))
        .order(egui::Order::Foreground)
        .interactable(false)
        .show(ctx, |ui| {
            egui::Frame::NONE
                .fill(color)
                .stroke(stroke)
                .corner_radius((12.0 * scale) as u8)
                .inner_margin(Margin::symmetric(
                    (20.0 * scale) as i8,
                    (12.0 * scale) as i8,
                ))
                .show(ui, |ui| {
                    ui.set_max_width(920.0 * scale);
                    ui.label(RichText::new(&toast.message).size(15.0 * scale).color(TEXT));
                });
        });
}

//! Profile selection screen.

use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontId, Id, Rect, RichText, Sense, Stroke,
    UiBuilder, Vec2,
};
use std::time::Duration;

use crate::app::CouchDeckApp;
use crate::model::Profile;
use crate::ui::{
    self, paint_texture, primary_button, secondary_button, truncate, ACCENT, CARD, MUTED, OK, TEXT,
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

    let content = Rect::from_min_max(
        egui::pos2(screen.left() + margin, header_rect.bottom() + 28.0 * scale),
        egui::pos2(
            screen.right() - margin,
            screen.bottom() - margin * 0.65 - 92.0 * scale,
        ),
    );
    let footer = Rect::from_min_max(
        egui::pos2(
            screen.left() + margin,
            screen.bottom() - margin * 0.65 - 76.0 * scale,
        ),
        egui::pos2(screen.right() - margin, screen.bottom() - margin * 0.65),
    );

    let profiles = app.library.profiles.clone();
    if app.profile_focus >= profiles.len() {
        app.profile_focus = profiles.len().saturating_sub(1);
    }
    draw_cards(ui, app, &profiles, content, scale);
    draw_footer(ui, app, footer, scale);
}

fn draw_cards(
    ui: &mut egui::Ui,
    app: &mut CouchDeckApp,
    profiles: &[Profile],
    rect: Rect,
    scale: f32,
) {
    let heading_rect = Rect::from_min_max(
        rect.min,
        egui::pos2(rect.right(), rect.top() + 86.0 * scale),
    );
    ui.scope_builder(
        UiBuilder::new()
            .max_rect(heading_rect)
            .id_salt("profiles-heading"),
        |ui| {
            ui.label(
                RichText::new("Profil wählen")
                    .size(34.0 * scale)
                    .strong()
                    .color(TEXT),
            );
            ui.label(
                RichText::new(
                    "Lokale Profile im selben Linux-Konto. Keine Cloud, keine Linux-Konten.",
                )
                .size(14.0 * scale)
                .color(MUTED),
            );
        },
    );

    let grid = Rect::from_min_max(
        egui::pos2(rect.left(), heading_rect.bottom() + 12.0 * scale),
        rect.max,
    );
    if profiles.is_empty() {
        ui.painter().text(
            grid.center(),
            Align2::CENTER_CENTER,
            "Keine Profile vorhanden",
            FontId::proportional(22.0 * scale),
            MUTED,
        );
        return;
    }

    let columns = ((grid.width() / (230.0 * scale)).floor() as usize).clamp(2, 6);
    let gap = 22.0 * scale;
    let card_w = ((grid.width() - gap * (columns.saturating_sub(1)) as f32) / columns as f32)
        .min(280.0 * scale)
        .max(150.0 * scale);
    let card_h = (card_w * 1.12).max(190.0 * scale);
    let rows_visible = (((grid.height() + gap) / (card_h + gap)).floor() as usize).max(1);
    let rows_total = profiles.len().div_ceil(columns);
    let selected_row = app.profile_focus / columns;
    let page_start = (selected_row / rows_visible) * rows_visible;
    let used_w = columns as f32 * card_w + gap * columns.saturating_sub(1) as f32;
    let offset_x = (grid.width() - used_w) * 0.5;
    let page_count = rows_total.div_ceil(rows_visible).max(1);
    let page = selected_row / rows_visible + 1;

    let elapsed = std::time::Instant::now()
        .saturating_duration_since(app.focus_changed_at)
        .as_secs_f32();
    let t = if app.settings.animations {
        (elapsed / 0.18).clamp(0.0, 1.0)
    } else {
        1.0
    };
    if t < 1.0 {
        ui.ctx().request_repaint_after(Duration::from_millis(16));
    }
    let eased = 1.0 - (1.0 - t).powi(3);

    ui.scope_builder(
        UiBuilder::new().max_rect(grid).id_salt("profiles-grid"),
        |ui| {
            ui.set_clip_rect(grid.expand(gap));
            let last_row = (page_start + rows_visible).min(rows_total);
            for row in page_start..last_row {
                for col in 0..columns {
                    let index = row * columns + col;
                    let Some(profile) = profiles.get(index) else {
                        continue;
                    };
                    let normal = Rect::from_min_size(
                        egui::pos2(
                            grid.left() + offset_x + col as f32 * (card_w + gap),
                            grid.top() + (row - page_start) as f32 * (card_h + gap),
                        ),
                        Vec2::new(card_w, card_h),
                    );
                    let response = ui
                        .interact(normal, Id::new(("profile", &profile.id)), Sense::click())
                        .on_hover_cursor(egui::CursorIcon::PointingHand);
                    if response.double_clicked() {
                        app.profile_focus = index;
                        app.activate_profile(index);
                        return;
                    } else if response.clicked() {
                        app.profile_focus = index;
                        app.focus_changed_at = std::time::Instant::now();
                    }
                    let focused = index == app.profile_focus;
                    let rect = if focused {
                        Rect::from_center_size(
                            normal.center(),
                            normal.size() * (1.0 + 0.025 * eased),
                        )
                    } else {
                        normal
                    };
                    draw_profile_card(ui, app, profile, rect, focused, scale);
                }
            }
            if page_count > 1 {
                ui.painter().text(
                    egui::pos2(grid.right(), grid.top() - 4.0 * scale),
                    Align2::RIGHT_BOTTOM,
                    format!("Seite {page} / {page_count}"),
                    FontId::proportional(13.0 * scale),
                    MUTED,
                );
            }
        },
    );
}

fn draw_profile_card(
    ui: &mut egui::Ui,
    app: &mut CouchDeckApp,
    profile: &Profile,
    rect: Rect,
    focused: bool,
    scale: f32,
) {
    let painter = ui.painter().with_clip_rect(ui.clip_rect());
    let corner = (18.0 * scale) as u8;
    painter.rect_filled(
        rect.translate(Vec2::new(0.0, 6.0 * scale)),
        CornerRadius::same(corner),
        Color32::from_black_alpha(120),
    );
    painter.rect_filled(rect, CornerRadius::same(corner), CARD);
    if focused {
        painter.rect_stroke(
            rect.expand(2.0 * scale),
            CornerRadius::same(corner),
            Stroke::new(3.0 * scale, ACCENT),
            egui::StrokeKind::Outside,
        );
    } else {
        painter.rect_stroke(
            rect,
            CornerRadius::same(corner),
            Stroke::new(1.0, Color32::from_rgb(48, 57, 72)),
            egui::StrokeKind::Inside,
        );
    }

    let avatar_side = (rect.width() * 0.42).min(120.0 * scale);
    let avatar_rect = Rect::from_center_size(
        egui::pos2(
            rect.center().x,
            rect.top() + avatar_side * 0.62 + 18.0 * scale,
        ),
        Vec2::splat(avatar_side),
    );
    let avatar_path = app.profile_avatar_path(profile);
    let avatar_tex = avatar_path.and_then(|path| {
        app.artwork.texture(
            ui.ctx(),
            &app.paths,
            &path,
            crate::artwork::ArtworkKind::Avatar,
        )
    });
    if let Some(texture) = avatar_tex {
        paint_texture(ui, &texture, avatar_rect, (avatar_side * 0.5) as u8);
    } else {
        painter.circle_filled(
            avatar_rect.center(),
            avatar_side * 0.5,
            Color32::from_rgb(45, 56, 74),
        );
        let initial = profile
            .name
            .chars()
            .next()
            .map(|c| c.to_uppercase().to_string())
            .unwrap_or_else(|| "?".to_owned());
        painter.text(
            avatar_rect.center(),
            Align2::CENTER_CENTER,
            initial,
            FontId::proportional(avatar_side * 0.42),
            TEXT,
        );
    }

    painter.text(
        egui::pos2(rect.center().x, avatar_rect.bottom() + 18.0 * scale),
        Align2::CENTER_TOP,
        truncate(&profile.name, 22),
        FontId::proportional(19.0 * scale),
        TEXT,
    );
    let role_color = if profile.is_admin() { ACCENT } else { MUTED };
    painter.text(
        egui::pos2(rect.center().x, avatar_rect.bottom() + 48.0 * scale),
        Align2::CENTER_TOP,
        profile.role.label(),
        FontId::proportional(13.0 * scale),
        role_color,
    );

    if app.active_profile_id() == Some(profile.id.as_str()) {
        let badge = Rect::from_min_size(
            egui::pos2(rect.left() + 10.0 * scale, rect.top() + 10.0 * scale),
            Vec2::new(78.0 * scale, 25.0 * scale),
        );
        painter.rect_filled(
            badge,
            CornerRadius::same((5.0 * scale) as u8),
            Color32::from_rgb(28, 92, 58),
        );
        painter.text(
            badge.center(),
            Align2::CENTER_CENTER,
            "Aktiv",
            FontId::proportional(12.0 * scale),
            OK,
        );
    }
}

fn draw_footer(ui: &mut egui::Ui, app: &mut CouchDeckApp, rect: Rect, scale: f32) {
    ui.scope_builder(
        UiBuilder::new().max_rect(rect).id_salt("profiles-footer"),
        |ui| {
            ui.horizontal_centered(|ui| {
                if app.settings.show_controller_hints {
                    ui.label(
                        RichText::new("LB/RB wechseln   ·   A Profil wählen   ·   B zurück")
                            .size(14.0 * scale)
                            .color(MUTED),
                    );
                }
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .add_sized(
                            Vec2::new(210.0 * scale, 50.0 * scale),
                            primary_button("Profil wählen  (A)"),
                        )
                        .clicked()
                    {
                        app.activate_profile(app.profile_focus);
                    }
                    if ui
                        .add_sized(
                            Vec2::new(150.0 * scale, 50.0 * scale),
                            secondary_button("Zurück  (B)"),
                        )
                        .clicked()
                    {
                        app.screen = crate::app::Screen::Library;
                    }
                });
            });
        },
    );
}

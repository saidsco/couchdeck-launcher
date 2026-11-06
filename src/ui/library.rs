//! Fullscreen library grid.

use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontId, Id, Rect, RichText, Sense, Stroke,
    UiBuilder, Vec2,
};
use std::time::Duration;

use crate::app::CouchDeckApp;
use crate::model::GameEntry;
use crate::ui::{
    self, paint_texture, primary_button, secondary_button, truncate, TileImage, ACCENT, CARD,
    MUTED, TEXT,
};

pub fn draw(ui: &mut egui::Ui, app: &mut CouchDeckApp) {
    let ctx = ui.ctx().clone();
    let scale = ui::ui_scale(&ctx);
    let screen = ctx.screen_rect();
    ui::paint_background(app, ui);

    let margin = 40.0 * scale;
    let header_h = 68.0 * scale;
    let footer_h = 76.0 * scale;
    let header_rect = Rect::from_min_max(
        egui::pos2(screen.left() + margin, screen.top() + margin * 0.65),
        egui::pos2(
            screen.right() - margin,
            screen.top() + margin * 0.65 + header_h,
        ),
    );
    ui::header(ui, app, header_rect, scale);

    let footer_rect = Rect::from_min_max(
        egui::pos2(
            screen.left() + margin,
            screen.bottom() - margin * 0.65 - footer_h,
        ),
        egui::pos2(screen.right() - margin, screen.bottom() - margin * 0.65),
    );
    let grid_rect = Rect::from_min_max(
        egui::pos2(header_rect.left(), header_rect.bottom() + 20.0 * scale),
        egui::pos2(footer_rect.right(), footer_rect.top() - 16.0 * scale),
    );

    let games = app.visible_games();
    let count = games.len();
    if app.focus >= count {
        app.focus = count.saturating_sub(1);
    }
    if games.is_empty() {
        draw_empty(ui, app, grid_rect, scale);
        draw_footer(ui, app, footer_rect, scale, None, 0, 0);
    } else {
        let (page, page_count) = draw_grid(ui, app, &games, grid_rect, scale);
        let focused = games.get(app.focus).cloned();
        draw_footer(ui, app, footer_rect, scale, focused, page, page_count);
    }
}

struct GridMetrics {
    columns: usize,
    tile: Vec2,
    gap: f32,
    rows_visible: usize,
    page_start_row: usize,
    page_count: usize,
    page: usize,
    used_width: f32,
}

fn grid_metrics(
    app: &CouchDeckApp,
    ctx: &egui::Context,
    games: &[GameEntry],
    rect: Rect,
    scale: f32,
) -> GridMetrics {
    let columns = app.column_count(ctx).max(1);
    let gap = 20.0 * scale;
    let mut tile_w =
        ((rect.width() - gap * (columns.saturating_sub(1)) as f32) / columns as f32).max(80.0);
    let mut tile_h = tile_w * 4.0 / 3.0;
    if tile_h > rect.height() {
        tile_h = rect.height();
        tile_w = tile_h * 0.75;
    }
    let rows_visible = (((rect.height() + gap) / (tile_h + gap)).floor() as usize).max(1);
    let rows_total = games.len().div_ceil(columns);
    let page_count = rows_total.div_ceil(rows_visible).max(1);
    let focus_row = (app.focus / columns).min(rows_total.saturating_sub(1));
    let page_start_row = (focus_row / rows_visible) * rows_visible;
    let page = focus_row / rows_visible + 1;
    let used_width = columns as f32 * tile_w + gap * columns.saturating_sub(1) as f32;
    GridMetrics {
        columns,
        tile: Vec2::new(tile_w, tile_h),
        gap,
        rows_visible,
        page_start_row,
        page_count,
        page,
        used_width,
    }
}

fn draw_grid(
    ui: &mut egui::Ui,
    app: &mut CouchDeckApp,
    games: &[GameEntry],
    rect: Rect,
    scale: f32,
) -> (usize, usize) {
    let metrics = grid_metrics(app, ui.ctx(), games, rect, scale);
    let offset_x = (rect.width() - metrics.used_width) * 0.5;
    let now = std::time::Instant::now();
    let elapsed = now
        .saturating_duration_since(app.focus_changed_at)
        .as_secs_f32();
    let t = if app.settings.animations {
        (elapsed / 0.18).clamp(0.0, 1.0)
    } else {
        1.0
    };
    let eased = 1.0 - (1.0 - t).powi(3);
    if t < 1.0 {
        ui.ctx().request_repaint_after(Duration::from_millis(16));
    }

    let info = ui
        .scope_builder(
            UiBuilder::new().max_rect(rect).id_salt("library-grid"),
            |ui| {
                ui.set_clip_rect(rect.expand(metrics.gap));
                let last_row = (metrics.page_start_row + metrics.rows_visible)
                    .min(games.len().div_ceil(metrics.columns));
                let mut focused_draw: Option<(GameEntry, Rect, bool)> = None;
                for row in metrics.page_start_row..last_row {
                    for col in 0..metrics.columns {
                        let index = row * metrics.columns + col;
                        if index >= games.len() {
                            break;
                        }
                        let game = &games[index];
                        let normal = Rect::from_min_size(
                            egui::pos2(
                                rect.left()
                                    + offset_x
                                    + col as f32 * (metrics.tile.x + metrics.gap),
                                rect.top()
                                    + (row - metrics.page_start_row) as f32
                                        * (metrics.tile.y + metrics.gap),
                            ),
                            metrics.tile,
                        );
                        let response = ui
                            .interact(normal, Id::new(("game-tile", &game.id)), Sense::click())
                            .on_hover_cursor(egui::CursorIcon::PointingHand);
                        if response.double_clicked() {
                            app.set_focus(index, games.len());
                            app.launch_game(game);
                        } else if response.clicked() {
                            app.set_focus(index, games.len());
                        }
                        let focused = index == app.focus;
                        let hovered = response.hovered();
                        if focused {
                            let scale_factor = 1.0 + 0.055 * eased;
                            let grown = Rect::from_center_size(
                                normal.center(),
                                normal.size() * scale_factor,
                            );
                            focused_draw = Some((game.clone(), grown, hovered));
                        } else {
                            draw_tile(ui, app, game, normal, false, hovered, scale, 0.0);
                        }
                    }
                }
                if let Some((game, tile, hovered)) = focused_draw {
                    draw_tile(ui, app, &game, tile, true, hovered, scale, eased);
                }

                if metrics.page_count > 1 {
                    let label = app.tf(
                        "library-page",
                        &[
                            ("page", metrics.page.into()),
                            ("count", metrics.page_count.into()),
                        ],
                    );
                    ui.painter().text(
                        egui::pos2(rect.right(), rect.top() - 6.0 * scale),
                        Align2::RIGHT_BOTTOM,
                        label,
                        FontId::proportional(13.0 * scale),
                        MUTED,
                    );
                }
                (metrics.page, metrics.page_count)
            },
        )
        .inner;
    info
}

#[allow(clippy::too_many_arguments)]
fn draw_tile(
    ui: &mut egui::Ui,
    app: &mut CouchDeckApp,
    game: &GameEntry,
    rect: Rect,
    focused: bool,
    hovered: bool,
    scale: f32,
    focus_ease: f32,
) {
    let painter = ui.painter().with_clip_rect(ui.clip_rect());
    let corner = (14.0 * scale) as u8;
    let shadow_offset = Vec2::new(0.0, (6.0 + 8.0 * focus_ease) * scale);
    painter.rect_filled(
        rect.translate(shadow_offset),
        CornerRadius::same(corner),
        Color32::from_black_alpha(150),
    );

    painter.rect_filled(rect, CornerRadius::same(corner), CARD);

    let image = ui::game_tile_image(app, ui.ctx(), game);
    match image {
        Some(TileImage::Cover(texture)) => {
            paint_texture(ui, &texture, rect, corner);
        }
        Some(TileImage::Icon(texture)) => {
            painter.rect_filled(
                rect,
                CornerRadius::same(corner),
                Color32::from_rgb(20, 24, 33),
            );
            let side = rect.width().min(rect.height()) * 0.56;
            let icon_rect = Rect::from_center_size(rect.center(), Vec2::splat(side));
            paint_texture(ui, &texture, icon_rect, (8.0 * scale) as u8);
        }
        None => {
            let top = Color32::from_rgb(35, 43, 58);
            let bottom = Color32::from_rgb(18, 22, 30);
            ui::paint_vertical_gradient(&painter, rect, top, bottom);
            let initial = game
                .title()
                .chars()
                .next()
                .map(|c| c.to_uppercase().to_string())
                .unwrap_or_else(|| "?".to_owned());
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                initial,
                FontId::proportional(64.0 * scale),
                Color32::from_rgb(75, 88, 108),
            );
        }
    }

    // Readability gradient + title.
    let title_h = (72.0 * scale).min(rect.height() * 0.36);
    let title_rect = Rect::from_min_max(
        egui::pos2(rect.left(), rect.bottom() - title_h),
        rect.right_bottom(),
    );
    ui::paint_vertical_gradient(
        &painter,
        title_rect,
        Color32::TRANSPARENT,
        Color32::from_black_alpha(225),
    );
    let title_chars = (((rect.width() - 26.0 * scale) / (7.0 * scale)).max(6.0)) as usize;
    painter.text(
        egui::pos2(rect.left() + 13.0 * scale, rect.bottom() - 13.0 * scale),
        Align2::LEFT_BOTTOM,
        truncate(&game.title(), title_chars),
        FontId::proportional(15.5 * scale),
        TEXT,
    );

    if !game.desktop_exists() {
        let badge = Rect::from_min_size(
            egui::pos2(rect.left() + 9.0 * scale, rect.top() + 9.0 * scale),
            Vec2::new(94.0 * scale, 25.0 * scale),
        );
        painter.rect_filled(
            badge,
            CornerRadius::same((6.0 * scale) as u8),
            Color32::from_rgb(150, 40, 40),
        );
        painter.text(
            badge.center(),
            Align2::CENTER_CENTER,
            app.t("library-file-missing"),
            FontId::proportional(12.0 * scale),
            Color32::WHITE,
        );
    }

    if focused {
        let glow = Stroke::new(
            7.0 * scale,
            Color32::from_rgba_unmultiplied(255, 118, 64, 48),
        );
        painter.rect_stroke(
            rect.expand(2.0 * scale),
            CornerRadius::same(corner),
            glow,
            egui::StrokeKind::Outside,
        );
        painter.rect_stroke(
            rect.expand(1.0 * scale),
            CornerRadius::same(corner),
            Stroke::new(3.0 * scale, ACCENT),
            egui::StrokeKind::Outside,
        );
    } else if hovered {
        painter.rect_stroke(
            rect,
            CornerRadius::same(corner),
            Stroke::new(1.5 * scale, Color32::from_rgb(100, 112, 134)),
            egui::StrokeKind::Inside,
        );
    }
}

fn draw_empty(ui: &mut egui::Ui, app: &mut CouchDeckApp, rect: Rect, scale: f32) {
    let card_size = Vec2::new(rect.width().min(680.0 * scale), 300.0 * scale);
    let card = Rect::from_center_size(rect.center(), card_size);
    let painter = ui.painter();
    painter.rect_filled(
        card,
        CornerRadius::same((22.0 * scale) as u8),
        Color32::from_black_alpha(95),
    );
    painter.rect_stroke(
        card,
        CornerRadius::same((22.0 * scale) as u8),
        Stroke::new(1.0, Color32::from_rgb(48, 57, 72)),
        egui::StrokeKind::Inside,
    );
    painter.text(
        egui::pos2(card.center().x, card.top() + 58.0 * scale),
        Align2::CENTER_CENTER,
        app.t("library-empty-title"),
        FontId::proportional(27.0 * scale),
        TEXT,
    );
    painter.text(
        egui::pos2(card.center().x, card.top() + 102.0 * scale),
        Align2::CENTER_CENTER,
        if app.is_admin() {
            app.t("library-empty-admin")
        } else {
            app.t("library-empty-player")
        },
        FontId::proportional(15.0 * scale),
        MUTED,
    );

    ui.scope_builder(
        UiBuilder::new()
            .max_rect(Rect::from_center_size(
                egui::pos2(card.center().x, card.bottom() - 66.0 * scale),
                Vec2::new(card.width() - 80.0 * scale, 52.0 * scale),
            ))
            .id_salt("empty-actions"),
        |ui| {
            ui.horizontal_centered(|ui| {
                if app.is_admin() {
                    if ui
                        .add_sized(
                            Vec2::new(230.0 * scale, 46.0 * scale),
                            primary_button(app.t("library-import")),
                        )
                        .clicked()
                    {
                        app.open_import();
                    }
                    if ui
                        .add_sized(
                            Vec2::new(190.0 * scale, 46.0 * scale),
                            secondary_button(app.t("library-profile-switch")),
                        )
                        .clicked()
                    {
                        app.open_profiles();
                    }
                } else if ui
                    .add_sized(
                        Vec2::new(220.0 * scale, 46.0 * scale),
                        secondary_button(app.t("library-profile-switch")),
                    )
                    .clicked()
                {
                    app.open_profiles();
                }
            });
        },
    );
}

fn draw_footer(
    ui: &mut egui::Ui,
    app: &mut CouchDeckApp,
    rect: Rect,
    scale: f32,
    game: Option<GameEntry>,
    page: usize,
    page_count: usize,
) {
    ui.scope_builder(
        UiBuilder::new().max_rect(rect).id_salt("library-footer"),
        |ui| {
            ui.horizontal_centered(|ui| {
                let text_rect = Rect::from_min_max(
                    rect.min,
                    egui::pos2(rect.right() - 390.0 * scale, rect.bottom()),
                );
                ui.scope_builder(
                    UiBuilder::new()
                        .max_rect(text_rect)
                        .layout(egui::Layout::top_down(egui::Align::LEFT))
                        .id_salt("footer-text"),
                    |ui| {
                        if let Some(game) = &game {
                            ui.label(
                                RichText::new(truncate(&game.title(), 52))
                                    .size(23.0 * scale)
                                    .strong()
                                    .color(TEXT),
                            );
                            let mut hint = if app.settings.show_controller_hints {
                                app.t("library-hints")
                            } else {
                                String::new()
                            };
                            if !game.desktop_exists() {
                                hint = app.t("library-missing-launch");
                            } else if page_count > 1 {
                                if !hint.is_empty() {
                                    hint.push_str("   ·   ");
                                }
                                hint.push_str(&app.tf(
                                    "library-page",
                                    &[("page", page.into()), ("count", page_count.into())],
                                ));
                            }
                            if !hint.is_empty() {
                                ui.label(RichText::new(hint).size(13.0 * scale).color(MUTED));
                            }
                        } else if app.is_admin() {
                            ui.label(
                                RichText::new(app.t("library-no-games"))
                                    .size(23.0 * scale)
                                    .strong()
                                    .color(TEXT),
                            );
                            if app.settings.show_controller_hints {
                                ui.label(
                                    RichText::new(app.t("library-no-games-hint"))
                                        .size(13.0 * scale)
                                        .color(MUTED),
                                );
                            }
                        } else {
                            ui.label(
                                RichText::new(app.t("library-no-games-profile"))
                                    .size(23.0 * scale)
                                    .strong()
                                    .color(TEXT),
                            );
                            if app.settings.show_controller_hints {
                                ui.label(
                                    RichText::new(app.t("library-no-games-profile-hint"))
                                        .size(13.0 * scale)
                                        .color(MUTED),
                                );
                            }
                        }
                    },
                );

                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    if let Some(game) = &game {
                        let enabled = game.desktop_exists();
                        let button = if enabled {
                            primary_button(app.t("library-start"))
                        } else {
                            egui::Button::new(
                                RichText::new(app.t("library-file-missing")).color(MUTED),
                            )
                            .fill(CARD)
                            .stroke(Stroke::new(1.0, Color32::from_rgb(70, 70, 70)))
                            .corner_radius(12)
                        };
                        let response = ui.add_sized(Vec2::new(210.0 * scale, 50.0 * scale), button);
                        if response.clicked() && enabled {
                            app.launch_game(game);
                        }
                        if ui
                            .add_sized(
                                Vec2::new(150.0 * scale, 50.0 * scale),
                                secondary_button(app.t("common-menu")),
                            )
                            .clicked()
                        {
                            app.open_menu();
                        }
                    } else if app.is_admin() {
                        if ui
                            .add_sized(
                                Vec2::new(210.0 * scale, 50.0 * scale),
                                primary_button(app.t("library-import-action")),
                            )
                            .clicked()
                        {
                            app.open_import();
                        }
                    } else if ui
                        .add_sized(
                            Vec2::new(210.0 * scale, 50.0 * scale),
                            secondary_button(app.t("library-profile-switch")),
                        )
                        .clicked()
                    {
                        app.open_profiles();
                    }
                });
            });
        },
    );
}

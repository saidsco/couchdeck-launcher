//! Child time-budget dialogs: exhausted screen and parent extra-time grant.

use eframe::egui::{self, Color32, RichText, Vec2};

use crate::app::{CouchDeckApp, ExtraTimeDialog, Overlay, TimeExpiredState};
use crate::ui::{
    self, dialogs::show_modal, primary_button, secondary_button, MUTED, TEXT, WARNING,
};

pub fn draw_time_expired(
    ctx: &egui::Context,
    app: &mut CouchDeckApp,
    mut state: TimeExpiredState,
) -> Option<Overlay> {
    let scale = ui::ui_scale(ctx);
    let mut request = false;
    let mut close = false;
    show_modal(ctx, "time-expired-modal", scale, |ui| {
        ui.set_min_width(620.0 * scale);
        ui.label(
            RichText::new(app.t("time-expired-title"))
                .size(28.0 * scale)
                .strong()
                .color(WARNING),
        );
        let name = app
            .library
            .profiles
            .iter()
            .find(|profile| profile.id == state.profile_id)
            .map(|profile| profile.name.clone())
            .unwrap_or_else(|| app.t("common-child"));
        ui.label(
            RichText::new(app.tf("time-expired-message", &[("name", name.into())]))
                .size(15.0 * scale)
                .color(TEXT),
        );
        ui.label(
            RichText::new(app.t("time-expired-running"))
                .size(13.0 * scale)
                .color(MUTED),
        );
        ui.add_space(12.0 * scale);
        for (index, label) in [
            (0usize, app.t("time-expired-request")),
            (1usize, app.t("time-expired-back")),
        ] {
            let selected = state.focus == index;
            let response = ui.add_sized(
                Vec2::new(600.0 * scale, 48.0 * scale),
                if selected {
                    primary_button(label)
                } else {
                    secondary_button(label)
                },
            );
            if response.clicked() {
                state.focus = index;
                if index == 0 {
                    request = true;
                } else {
                    close = true;
                }
            }
        }
        ui.add_space(6.0 * scale);
        ui.label(
            RichText::new(app.t("time-expired-pin-hint"))
                .size(12.0 * scale)
                .color(MUTED),
        );
    });

    if request {
        app.request_extra_time();
        None
    } else if close {
        None
    } else {
        Some(Overlay::TimeExpired(state))
    }
}

pub fn draw_extra_time(
    ctx: &egui::Context,
    app: &mut CouchDeckApp,
    mut dialog: ExtraTimeDialog,
) -> Option<Overlay> {
    let scale = ui::ui_scale(ctx);
    let profile = app
        .library
        .profiles
        .iter()
        .find(|profile| profile.id == dialog.profile_id)
        .cloned();
    let profile = profile?;
    let status = app.time_status_for(&profile);
    let mut close = false;

    show_modal(ctx, "extra-time-modal", scale, |ui| {
        ui.set_min_width(620.0 * scale);
        ui.label(
            RichText::new(app.t("time-extra-title"))
                .size(28.0 * scale)
                .strong()
                .color(TEXT),
        );
        ui.label(
            RichText::new(app.tf(
                "time-extra-status",
                &[
                    ("name", profile.name.clone().into()),
                    ("bonus", status.bonus_minutes.into()),
                    ("remaining", status.remaining_minutes.into()),
                ],
            ))
            .size(13.0 * scale)
            .color(MUTED),
        );
        ui.add_space(10.0 * scale);

        if dialog.custom_mode {
            ui.label(
                RichText::new(app.t("time-extra-custom"))
                    .size(18.0 * scale)
                    .strong()
                    .color(TEXT),
            );
            ui.label(
                RichText::new(app.t("time-extra-custom-hint"))
                    .size(12.0 * scale)
                    .color(MUTED),
            );
            ui.horizontal(|ui| {
                if ui
                    .add_sized(
                        Vec2::new(90.0 * scale, 48.0 * scale),
                        secondary_button("-15"),
                    )
                    .clicked()
                {
                    dialog.custom_minutes = dialog.custom_minutes.saturating_sub(15).max(5);
                }
                if ui
                    .add_sized(
                        Vec2::new(90.0 * scale, 48.0 * scale),
                        secondary_button("-5"),
                    )
                    .clicked()
                {
                    dialog.custom_minutes = dialog.custom_minutes.saturating_sub(5).max(5);
                }
                ui.add_sized(
                    Vec2::new(170.0 * scale, 48.0 * scale),
                    egui::Button::new(
                        RichText::new(app.tf(
                            "time-extra-minutes",
                            &[("minutes", dialog.custom_minutes.into())],
                        ))
                        .size(22.0 * scale)
                        .strong()
                        .color(TEXT),
                    )
                    .fill(Color32::from_rgb(30, 37, 49)),
                );
                if ui
                    .add_sized(
                        Vec2::new(90.0 * scale, 48.0 * scale),
                        secondary_button("+5"),
                    )
                    .clicked()
                {
                    dialog.custom_minutes = (dialog.custom_minutes + 5).min(240);
                }
                if ui
                    .add_sized(
                        Vec2::new(90.0 * scale, 48.0 * scale),
                        secondary_button("+15"),
                    )
                    .clicked()
                {
                    dialog.custom_minutes = (dialog.custom_minutes + 15).min(240);
                }
            });
            ui.add_space(10.0 * scale);
            ui.horizontal(|ui| {
                if ui
                    .add_sized(
                        Vec2::new(250.0 * scale, 48.0 * scale),
                        primary_button(app.t("time-extra-grant")),
                    )
                    .clicked()
                {
                    match app.grant_extra_minutes(&dialog.profile_id, dialog.custom_minutes) {
                        Ok(()) => close = true,
                        Err(err) => dialog.error = Some(err),
                    }
                }
                if ui
                    .add_sized(
                        Vec2::new(200.0 * scale, 48.0 * scale),
                        secondary_button(app.t("time-extra-back")),
                    )
                    .clicked()
                {
                    dialog.custom_mode = false;
                }
                if ui
                    .add_sized(
                        Vec2::new(140.0 * scale, 48.0 * scale),
                        secondary_button(app.t("time-extra-cancel")),
                    )
                    .clicked()
                {
                    close = true;
                }
            });
        } else {
            let options = [
                (0usize, app.t("time-extra-option-5")),
                (1usize, app.t("time-extra-option-15")),
                (2usize, app.t("time-extra-option-30")),
                (3usize, app.t("time-extra-option-custom")),
            ];
            for (index, label) in options {
                let selected = dialog.focus == index;
                let response = ui.add_sized(
                    Vec2::new(600.0 * scale, 48.0 * scale),
                    if selected {
                        primary_button(label)
                    } else {
                        secondary_button(label)
                    },
                );
                if response.clicked() {
                    dialog.focus = index;
                    match index {
                        0 => match app.grant_extra_minutes(&dialog.profile_id, 5) {
                            Ok(()) => close = true,
                            Err(err) => dialog.error = Some(err),
                        },
                        1 => match app.grant_extra_minutes(&dialog.profile_id, 15) {
                            Ok(()) => close = true,
                            Err(err) => dialog.error = Some(err),
                        },
                        2 => match app.grant_extra_minutes(&dialog.profile_id, 30) {
                            Ok(()) => close = true,
                            Err(err) => dialog.error = Some(err),
                        },
                        _ => dialog.custom_mode = true,
                    }
                }
            }
            ui.add_space(8.0 * scale);
            if ui
                .add_sized(
                    Vec2::new(200.0 * scale, 42.0 * scale),
                    secondary_button(app.t("time-extra-cancel")),
                )
                .clicked()
            {
                close = true;
            }
        }

        if let Some(error) = &dialog.error {
            ui.add_space(6.0 * scale);
            ui.label(RichText::new(error).size(13.0 * scale).color(ui::DANGER));
        }
    });

    if close {
        None
    } else {
        Some(Overlay::ExtraTime(dialog))
    }
}

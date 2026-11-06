//! Modal dialogs, file picker, import preview, PIN pad, launch overlay and the
//! fatal corrupt-data screen.

use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontId, Id, Margin, Rect, RichText, ScrollArea,
    Stroke, TextEdit, Vec2,
};
use std::path::PathBuf;

use crate::app::{
    ConfirmAction, ConfirmDialog, CouchDeckApp, FilePickerState, ImportState, MenuAction,
    MenuState, MessageDialog, MessageSeverity, Overlay, PinPad, PinPurpose,
};
use crate::desktop::{self, Severity};
use crate::ui::{
    self, danger_button, primary_button, secondary_button, truncate, ACCENT, CARD, DANGER, MUTED,
    OK, TEXT, WARNING,
};

enum OverlayAction {
    Menu(MenuAction),
    ImportSelected(Box<desktop::DesktopFile>),
    ImportOther,
    PickerNavigate(PathBuf),
    PickerConfirm(PathBuf),
    PickerParent,
    PickerOpenPath,
    Pin(usize),
    Confirm(ConfirmAction),
    Close,
}

enum Outcome {
    Keep(Overlay),
    Consume(Option<OverlayAction>),
}

pub fn draw_overlay(app: &mut CouchDeckApp, ctx: &egui::Context) {
    if matches!(app.overlay, Overlay::None) {
        return;
    }
    let overlay = std::mem::replace(&mut app.overlay, Overlay::None);
    let outcome = match overlay {
        Overlay::None => Outcome::Consume(None),
        Overlay::Menu(state) => draw_menu(ctx, app, state),
        Overlay::Import(state) => draw_import(ctx, app, state),
        Overlay::Picker(state) => draw_picker(ctx, app, state),
        Overlay::Pin(pad) => draw_pin(ctx, app, pad),
        Overlay::Confirm(dialog) => draw_confirm(ctx, app, dialog),
        Overlay::Message(dialog) => draw_message(ctx, app, dialog),
        Overlay::TimeExpired(state) => match crate::ui::time::draw_time_expired(ctx, app, state) {
            Some(overlay) => Outcome::Keep(overlay),
            None => Outcome::Consume(None),
        },
        Overlay::ExtraTime(dialog) => match crate::ui::time::draw_extra_time(ctx, app, dialog) {
            Some(overlay) => Outcome::Keep(overlay),
            None => Outcome::Consume(None),
        },
    };
    match outcome {
        Outcome::Keep(overlay) => app.overlay = overlay,
        Outcome::Consume(Some(action)) => {
            apply_action(app, ctx, action);
            ctx.request_repaint();
        }
        Outcome::Consume(None) => ctx.request_repaint(),
    }
}

fn apply_action(app: &mut CouchDeckApp, ctx: &egui::Context, action: OverlayAction) {
    match action {
        OverlayAction::Menu(action) => app.execute_menu_action(ctx, action),
        OverlayAction::ImportSelected(entry) => app.import_entry(&entry),
        OverlayAction::ImportOther => app.open_import_file_picker(),
        // These are handled inside draw_picker while the state is still
        // available and must never reach this dispatcher.
        OverlayAction::PickerNavigate(_)
        | OverlayAction::PickerConfirm(_)
        | OverlayAction::PickerParent
        | OverlayAction::PickerOpenPath => {
            debug_assert!(false, "picker action escaped draw_picker");
        }
        OverlayAction::Pin(index) => app.pin_press(ctx, index),
        OverlayAction::Confirm(action) => app.apply_confirm(action),
        OverlayAction::Close => {}
    }
}

fn modal_frame(scale: f32) -> egui::Frame {
    egui::Frame::NONE
        .fill(Color32::from_rgb(19, 23, 32))
        .stroke(Stroke::new(1.0, Color32::from_rgb(58, 68, 86)))
        .corner_radius(CornerRadius::same((20.0 * scale) as u8))
        .inner_margin(Margin::same((24.0 * scale) as i8))
}

pub(crate) fn show_modal<R>(
    ctx: &egui::Context,
    id: &'static str,
    scale: f32,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    egui::Modal::new(Id::new(id))
        .backdrop_color(Color32::from_black_alpha(205))
        .frame(modal_frame(scale))
        .show(ctx, add_contents)
        .inner
}

fn draw_menu(ctx: &egui::Context, app: &mut CouchDeckApp, mut state: MenuState) -> Outcome {
    let scale = ui::ui_scale(ctx);
    let mut clicked = None;
    show_modal(ctx, "menu-modal", scale, |ui| {
        ui.set_min_width(520.0 * scale);
        ui.label(
            RichText::new(app.t("menu-title"))
                .size(28.0 * scale)
                .strong()
                .color(TEXT),
        );
        ui.label(
            RichText::new(app.t("menu-hint"))
                .size(12.5 * scale)
                .color(MUTED),
        );
        ui.add_space(10.0 * scale);
        for (index, item) in state.items.iter().enumerate() {
            let focused = index == state.focus;
            let mut button =
                egui::Button::new(RichText::new(&item.label).size(17.0 * scale).color(
                    if focused {
                        Color32::from_rgb(24, 15, 8)
                    } else {
                        TEXT
                    },
                ))
                .corner_radius(10)
                .min_size(Vec2::new(470.0 * scale, 46.0 * scale));
            if focused {
                button = button.fill(ACCENT).stroke(Stroke::NONE);
            } else {
                button = button
                    .fill(Color32::from_rgb(28, 34, 45))
                    .stroke(Stroke::new(1.0, Color32::from_rgb(48, 58, 74)));
            }
            let response = ui.add(button);
            if response.hovered() && !focused {
                state.focus = index;
            }
            if response.clicked() {
                clicked = Some(item.action);
            }
        }
    });
    match clicked {
        Some(action) => Outcome::Consume(Some(OverlayAction::Menu(action))),
        None => Outcome::Keep(Overlay::Menu(state)),
    }
}

fn draw_import(ctx: &egui::Context, app: &mut CouchDeckApp, mut state: ImportState) -> Outcome {
    let scale = ui::ui_scale(ctx);
    let mut action: Option<OverlayAction> = None;
    show_modal(ctx, "import-modal", scale, |ui| {
        ui.set_min_width(980.0 * scale);
        ui.label(
            RichText::new(app.t("dialog-import-title"))
                .size(26.0 * scale)
                .strong()
                .color(TEXT),
        );
        ui.label(
            RichText::new(app.t("dialog-import-subtitle"))
                .size(13.0 * scale)
                .color(MUTED),
        );
        ui.add_space(10.0 * scale);

        let body_h = (ctx.screen_rect().height() * 0.55).max(300.0);
        ui.horizontal(|ui| {
            let list_w = 360.0 * scale;
            ui.allocate_ui_with_layout(
                Vec2::new(list_w, body_h),
                egui::Layout::top_down(Align::LEFT),
                |ui| {
                    ui.set_min_width(list_w);
                    ui.label(
                        RichText::new(app.tf(
                            "dialog-found-apps",
                            &[("count", state.entries.len().into())],
                        ))
                        .size(14.0 * scale)
                        .color(MUTED),
                    );
                    ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .max_height(body_h - 28.0 * scale)
                        .show(ui, |ui| {
                            if state.entries.is_empty() {
                                ui.label(
                                    RichText::new(format!(
                                        "{}\n{}",
                                        app.t("dialog-no-apps"),
                                        app.t("dialog-other-file")
                                    ))
                                    .color(MUTED),
                                );
                            }
                            for (index, entry) in state.entries.iter().enumerate() {
                                let already = app.library.games.iter().any(|game| {
                                    std::fs::canonicalize(&game.desktop_file)
                                        .unwrap_or_else(|_| game.desktop_file.clone())
                                        == std::fs::canonicalize(&entry.path)
                                            .unwrap_or_else(|_| entry.path.clone())
                                });
                                let mut label = truncate(&entry.name, 38);
                                if already {
                                    label.push_str(&format!(
                                        "  {}",
                                        app.t("dialog-already-imported")
                                    ));
                                }
                                let response = ui.add_sized(
                                    Vec2::new(ui.available_width(), 36.0 * scale),
                                    egui::Button::selectable(index == state.selected, label),
                                );
                                if index == state.selected {
                                    response.scroll_to_me(Some(Align::Center));
                                }
                                if response.clicked() {
                                    state.selected = index;
                                    state.focus_actions = false;
                                }
                            }
                        });
                },
            );
            ui.add_space(16.0 * scale);
            let preview_w = (ui.available_width() - 8.0 * scale).max(420.0);
            ui.allocate_ui_with_layout(
                Vec2::new(preview_w, body_h),
                egui::Layout::top_down(Align::LEFT),
                |ui| {
                    ui.set_min_width(preview_w.min(560.0 * scale));
                    if let Some(entry) = state.entries.get(state.selected) {
                        ui.label(
                            RichText::new(truncate(&entry.name, 60))
                                .size(22.0 * scale)
                                .strong()
                                .color(TEXT),
                        );
                        ui.label(
                            RichText::new(truncate(&entry.path.to_string_lossy(), 100))
                                .monospace()
                                .size(11.5 * scale)
                                .color(MUTED),
                        );
                        if !entry.executable_for_gio() {
                            ui.label(
                                RichText::new(app.t("dialog-no-exec-warning"))
                                    .size(13.0 * scale)
                                    .color(DANGER),
                            );
                        }
                        ui.add_space(6.0 * scale);
                        ui.label(
                            RichText::new(app.t("dialog-exec-label"))
                                .size(12.0 * scale)
                                .color(MUTED),
                        );
                        ui.add(
                            egui::Label::new(
                                RichText::new(
                                    entry.exec.clone().unwrap_or_else(|| app.t("admin-no-exec")),
                                )
                                .monospace()
                                .size(12.0 * scale)
                                .color(TEXT),
                            )
                            .wrap(),
                        );
                        if let Some(icon) = &entry.icon {
                            ui.label(
                                RichText::new(
                                    app.tf("dialog-icon", &[("icon", icon.clone().into())]),
                                )
                                .monospace()
                                .size(11.5 * scale)
                                .color(MUTED),
                            );
                        }
                        if let Some(try_exec) = &entry.try_exec {
                            ui.label(
                                RichText::new(
                                    app.tf("dialog-tryexec", &[("value", try_exec.clone().into())]),
                                )
                                .monospace()
                                .size(11.5 * scale)
                                .color(MUTED),
                            );
                        }
                        if let Some(workdir) = &entry.workdir {
                            ui.label(
                                RichText::new(
                                    app.tf("dialog-workdir", &[("value", workdir.clone().into())]),
                                )
                                .monospace()
                                .size(11.5 * scale)
                                .color(MUTED),
                            );
                        }
                        for diagnostic in desktop::analyze_entry(entry, &app.paths) {
                            ui.label(RichText::new(diagnostic.message).size(12.5 * scale).color(
                                match diagnostic.severity {
                                    Severity::Info => MUTED,
                                    Severity::Warning => WARNING,
                                    Severity::Error => DANGER,
                                },
                            ));
                        }
                    } else {
                        ui.label(RichText::new(app.t("dialog-choose-app")).color(MUTED));
                    }
                },
            );
        });

        if let Some(error) = &state.error {
            ui.label(RichText::new(error).color(DANGER).size(13.0 * scale));
        }
        ui.add_space(8.0 * scale);
        ui.horizontal(|ui| {
            let focused_action = |index: usize| state.focus_actions && state.action_index == index;
            let action_button = |label: &str, index: usize| {
                if focused_action(index) {
                    egui::Button::new(
                        RichText::new(label)
                            .strong()
                            .color(Color32::from_rgb(24, 15, 8)),
                    )
                    .fill(ACCENT)
                    .stroke(Stroke::NONE)
                    .corner_radius(12)
                } else {
                    secondary_button(label)
                }
            };
            let import = ui.add_sized(
                Vec2::new(220.0 * scale, 44.0 * scale),
                action_button(&app.t("dialog-import"), 0),
            );
            if import.clicked() {
                if let Some(entry) = state.entries.get(state.selected).cloned() {
                    action = Some(OverlayAction::ImportSelected(Box::new(entry)));
                }
            }
            let other = ui.add_sized(
                Vec2::new(220.0 * scale, 44.0 * scale),
                action_button(&app.t("dialog-other-file"), 1),
            );
            if other.clicked() {
                action = Some(OverlayAction::ImportOther);
            }
            let cancel = ui.add_sized(
                Vec2::new(160.0 * scale, 44.0 * scale),
                action_button(&app.t("dialog-cancel"), 2),
            );
            if cancel.clicked() {
                action = Some(OverlayAction::Close);
            }
        });
        ui.label(
            RichText::new(app.t("dialog-import-hint"))
                .size(12.0 * scale)
                .color(MUTED),
        );
    });
    match action {
        Some(action) => Outcome::Consume(Some(action)),
        None => Outcome::Keep(Overlay::Import(state)),
    }
}

fn draw_picker(ctx: &egui::Context, app: &mut CouchDeckApp, mut state: FilePickerState) -> Outcome {
    let scale = ui::ui_scale(ctx);
    let mut action: Option<OverlayAction> = None;
    show_modal(ctx, "picker-modal", scale, |ui| {
        ui.set_min_width(940.0 * scale);
        ui.label(
            RichText::new(app.t(state.purpose.title_key()))
                .size(25.0 * scale)
                .strong()
                .color(TEXT),
        );
        ui.label(
            RichText::new(app.t("dialog-picker-hint"))
                .size(12.5 * scale)
                .color(MUTED),
        );
        ui.add_space(8.0 * scale);
        ui.horizontal(|ui| {
            if ui
                .add_sized(
                    Vec2::new(170.0 * scale, 38.0 * scale),
                    secondary_button(app.t("dialog-picker-parent")),
                )
                .clicked()
            {
                action = Some(OverlayAction::PickerParent);
            }
            if ui
                .add_sized(
                    Vec2::new(120.0 * scale, 38.0 * scale),
                    secondary_button(app.t("dialog-picker-home")),
                )
                .clicked()
            {
                action = Some(OverlayAction::PickerNavigate(app.paths.home.clone()));
            }
            let downloads = app.paths.home.join("Downloads");
            if downloads.is_dir()
                && ui
                    .add_sized(
                        Vec2::new(130.0 * scale, 38.0 * scale),
                        secondary_button(app.t("dialog-picker-downloads")),
                    )
                    .clicked()
            {
                action = Some(OverlayAction::PickerNavigate(downloads));
            }
            let apps = app.paths.data_home.join("applications");
            if apps.is_dir()
                && ui
                    .add_sized(
                        Vec2::new(160.0 * scale, 38.0 * scale),
                        secondary_button(app.t("dialog-picker-apps")),
                    )
                    .clicked()
            {
                action = Some(OverlayAction::PickerNavigate(apps));
            }
        });
        ui.horizontal(|ui| {
            let response = ui.add(
                TextEdit::singleline(&mut state.path_input)
                    .desired_width(680.0 * scale)
                    .hint_text(app.t("dialog-picker-path")),
            );
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                action = Some(OverlayAction::PickerOpenPath);
            }
            if ui
                .add_sized(
                    Vec2::new(120.0 * scale, 38.0 * scale),
                    secondary_button(app.t("dialog-picker-open")),
                )
                .clicked()
            {
                action = Some(OverlayAction::PickerOpenPath);
            }
        });
        ui.label(
            RichText::new(state.dir.display().to_string())
                .monospace()
                .size(11.5 * scale)
                .color(MUTED),
        );
        if let Some(error) = &state.error {
            ui.label(RichText::new(error).size(13.0 * scale).color(DANGER));
        }
        ui.add_space(6.0 * scale);
        let list_h = (ctx.screen_rect().height() * 0.48).max(260.0);
        ScrollArea::vertical()
            .auto_shrink([false, false])
            .max_height(list_h)
            .show(ui, |ui| {
                for (index, entry) in state.entries.iter().enumerate() {
                    let prefix = if entry.is_dir {
                        app.t("dialog-picker-folder")
                    } else {
                        app.t("dialog-picker-file")
                    };
                    let label = format!("{prefix}  {}", truncate(&entry.name, 70));
                    let response = ui.add_sized(
                        Vec2::new(ui.available_width(), 34.0 * scale),
                        egui::Button::selectable(index == state.selected, label),
                    );
                    if index == state.selected {
                        response.scroll_to_me(Some(Align::Center));
                    }
                    if response.clicked() {
                        state.selected = index;
                        state.error = None;
                    }
                    if response.double_clicked() {
                        if entry.is_dir {
                            action = Some(OverlayAction::PickerNavigate(entry.path.clone()));
                        } else {
                            action = Some(OverlayAction::PickerConfirm(entry.path.clone()));
                        }
                    }
                }
                if state.entries.is_empty() {
                    ui.label(RichText::new(app.t("dialog-picker-empty")).color(MUTED));
                }
            });
        ui.add_space(8.0 * scale);
        ui.horizontal(|ui| {
            if ui
                .add_sized(
                    Vec2::new(160.0 * scale, 42.0 * scale),
                    secondary_button(app.t("dialog-cancel")),
                )
                .clicked()
            {
                action = Some(OverlayAction::Close);
            }
            if ui
                .add_sized(
                    Vec2::new(200.0 * scale, 42.0 * scale),
                    primary_button(app.t("dialog-picker-select")),
                )
                .clicked()
            {
                if let Some(entry) = state.entries.get(state.selected) {
                    if entry.is_dir {
                        action = Some(OverlayAction::PickerNavigate(entry.path.clone()));
                    } else {
                        action = Some(OverlayAction::PickerConfirm(entry.path.clone()));
                    }
                }
            }
        });
    });

    match action {
        Some(OverlayAction::PickerParent) => {
            // Rebuild the state and use the app's helper, which keeps the
            // purpose intact.
            app.overlay = Overlay::Picker(state);
            app.picker_parent();
            Outcome::Consume(None)
        }
        Some(OverlayAction::PickerOpenPath) => {
            app.overlay = Overlay::Picker(state);
            app.picker_open_path_input();
            Outcome::Consume(None)
        }
        Some(OverlayAction::PickerNavigate(path)) => {
            app.overlay = Overlay::Picker(state);
            app.picker_navigate(path);
            Outcome::Consume(None)
        }
        Some(OverlayAction::PickerConfirm(path)) => {
            app.overlay = Overlay::Picker(state);
            app.picker_confirm_path(path);
            Outcome::Consume(None)
        }
        Some(_) => Outcome::Consume(None),
        None => Outcome::Keep(Overlay::Picker(state)),
    }
}

fn draw_pin(ctx: &egui::Context, app: &mut CouchDeckApp, pad: PinPad) -> Outcome {
    let scale = ui::ui_scale(ctx);
    let mut pressed = None;
    show_modal(ctx, "pin-modal", scale, |ui| {
        ui.set_min_width(380.0 * scale);
        ui.label(
            RichText::new(app.t(pad.title_key()))
                .size(24.0 * scale)
                .strong()
                .color(TEXT),
        );
        let subtitle = match &pad.purpose {
            PinPurpose::Unlock { .. } => app.t("dialog-pin-unlock"),
            PinPurpose::Set { .. } => app.t("dialog-pin-set-subtitle"),
            PinPurpose::ConfirmSet { .. } => app.t("dialog-pin-confirm-subtitle"),
            PinPurpose::ParentAuth { .. } => app.t("dialog-pin-parent-subtitle"),
        };
        ui.label(RichText::new(subtitle).size(12.5 * scale).color(MUTED));
        ui.add_space(12.0 * scale);
        let display = if pad.digits.is_empty() {
            "—".to_owned()
        } else {
            pad.masked()
        };
        ui.label(
            RichText::new(display)
                .size(30.0 * scale)
                .monospace()
                .color(TEXT),
        );
        if let Some(error) = &pad.error {
            ui.label(RichText::new(error).size(13.0 * scale).color(DANGER));
        }
        ui.add_space(8.0 * scale);

        for row in 0..4 {
            ui.horizontal(|ui| {
                for col in 0..3 {
                    let index = row * 3 + col;
                    let label = match index {
                        9 => "<-".to_owned(),
                        10 => "0".to_owned(),
                        11 => "OK".to_owned(),
                        n => (n + 1).to_string(),
                    };
                    let focused = pad.focus == index;
                    let mut button =
                        egui::Button::new(RichText::new(label).size(21.0 * scale).strong().color(
                            if focused {
                                Color32::from_rgb(24, 15, 8)
                            } else {
                                TEXT
                            },
                        ))
                        .corner_radius(10)
                        .min_size(Vec2::new(105.0 * scale, 54.0 * scale));
                    if focused {
                        button = button.fill(ACCENT).stroke(Stroke::NONE);
                    } else {
                        button = button
                            .fill(Color32::from_rgb(30, 37, 49))
                            .stroke(Stroke::new(1.0, Color32::from_rgb(52, 62, 79)));
                    }
                    if ui.add(button).clicked() {
                        pressed = Some(index);
                    }
                }
            });
        }
        ui.add_space(8.0 * scale);
        ui.horizontal(|ui| {
            if ui
                .add_sized(
                    Vec2::new(200.0 * scale, 38.0 * scale),
                    secondary_button(app.t("dialog-cancel")),
                )
                .clicked()
            {
                pressed = Some(usize::MAX);
            }
            ui.label(
                RichText::new(app.t("dialog-pin-store-hint"))
                    .size(11.5 * scale)
                    .color(MUTED),
            );
        });
    });
    match pressed {
        Some(usize::MAX) => Outcome::Consume(None),
        Some(index) => Outcome::Consume(Some(OverlayAction::Pin(index))),
        None => Outcome::Keep(Overlay::Pin(pad)),
    }
}

fn draw_confirm(ctx: &egui::Context, app: &mut CouchDeckApp, dialog: ConfirmDialog) -> Outcome {
    let scale = ui::ui_scale(ctx);
    let mut action = None;
    show_modal(ctx, "confirm-modal", scale, |ui| {
        ui.set_min_width(560.0 * scale);
        ui.label(
            RichText::new(&dialog.title)
                .size(24.0 * scale)
                .strong()
                .color(TEXT),
        );
        ui.add_space(6.0 * scale);
        ui.label(
            RichText::new(&dialog.message)
                .size(14.0 * scale)
                .color(TEXT),
        );
        ui.add_space(14.0 * scale);
        ui.horizontal(|ui| {
            let cancel_label = app.t("dialog-confirm-cancel");
            let mut cancel =
                secondary_button(cancel_label).min_size(Vec2::new(180.0 * scale, 46.0 * scale));
            if dialog.focus == 0 {
                cancel = cancel.fill(Color32::from_rgb(50, 60, 76));
            }
            if ui.add(cancel).clicked() {
                action = Some(OverlayAction::Close);
            }
            let label = format!("{}  (A)", dialog.confirm_label);
            let mut confirm = if dialog.danger {
                danger_button(label)
            } else {
                primary_button(label)
            }
            .min_size(Vec2::new(260.0 * scale, 46.0 * scale));
            if dialog.focus == 1 {
                confirm = confirm
                    .fill(if dialog.danger {
                        Color32::from_rgb(190, 55, 55)
                    } else {
                        ACCENT
                    })
                    .stroke(Stroke::NONE);
            }
            if ui.add(confirm).clicked() {
                action = Some(OverlayAction::Confirm(dialog.action.clone()));
            }
        });
    });
    match action {
        Some(action) => Outcome::Consume(Some(action)),
        None => Outcome::Keep(Overlay::Confirm(dialog)),
    }
}

fn draw_message(ctx: &egui::Context, app: &CouchDeckApp, dialog: MessageDialog) -> Outcome {
    let scale = ui::ui_scale(ctx);
    let mut close = false;
    let (accent, label) = match dialog.severity {
        MessageSeverity::Info => (ACCENT, app.t("dialog-severity-info")),
        MessageSeverity::Warning => (WARNING, app.t("dialog-severity-warning")),
        MessageSeverity::Error => (DANGER, app.t("dialog-severity-error")),
    };
    show_modal(ctx, "message-modal", scale, |ui| {
        ui.set_min_width(620.0 * scale);
        ui.label(
            RichText::new(label)
                .size(12.5 * scale)
                .strong()
                .color(accent),
        );
        ui.label(
            RichText::new(&dialog.title)
                .size(24.0 * scale)
                .strong()
                .color(TEXT),
        );
        ui.add_space(6.0 * scale);
        ui.add(
            egui::Label::new(
                RichText::new(&dialog.message)
                    .size(14.0 * scale)
                    .color(TEXT),
            )
            .wrap(),
        );
        ui.add_space(12.0 * scale);
        if ui
            .add_sized(
                Vec2::new(190.0 * scale, 44.0 * scale),
                primary_button(format!("{}  (A)", dialog.close_label)),
            )
            .clicked()
        {
            close = true;
        }
    });
    if close {
        Outcome::Consume(None)
    } else {
        Outcome::Keep(Overlay::Message(dialog))
    }
}

pub fn draw_power(app: &CouchDeckApp, ctx: &egui::Context) {
    let Some(state) = app.system.state() else {
        return;
    };
    let scale = ui::ui_scale(ctx);
    show_modal(ctx, "power-modal", scale, |ui| {
        ui.set_min_width(560.0 * scale);
        if state.outcome.is_some() {
            ui.label(
                RichText::new(app.t("dialog-power-success"))
                    .size(24.0 * scale)
                    .strong()
                    .color(OK),
            );
            ui.label(
                RichText::new(app.t("dialog-power-quit"))
                    .size(13.0 * scale)
                    .color(MUTED),
            );
        } else {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(
                    RichText::new(app.t(state.action.progress_key()))
                        .size(23.0 * scale)
                        .strong()
                        .color(TEXT),
                );
            });
            ui.label(
                RichText::new(app.t("dialog-power-hint"))
                    .size(13.0 * scale)
                    .color(MUTED),
            );
            if state.started.elapsed().as_secs() > 5 {
                ui.label(
                    RichText::new(app.t("dialog-power-wait"))
                        .size(12.5 * scale)
                        .color(WARNING),
                );
            }
        }
    });
}

pub fn draw_launch(app: &mut CouchDeckApp, ctx: &egui::Context) {
    let Some(state) = &app.launch else {
        return;
    };
    let scale = ui::ui_scale(ctx);
    let mut dismiss = false;
    let mut minimize = false;
    show_modal(ctx, "launch-modal", scale, |ui| {
        ui.set_min_width(520.0 * scale);
        let pending = state.outcome.is_none();
        if pending {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(
                    RichText::new(app.tf(
                        "dialog-launch-starting",
                        &[("title", state.title.clone().into())],
                    ))
                    .size(23.0 * scale)
                    .strong()
                    .color(TEXT),
                );
            });
            ui.label(
                RichText::new(app.t("dialog-launch-gio-hint"))
                    .size(13.0 * scale)
                    .color(MUTED),
            );
            if state.started.elapsed().as_secs() > 4 {
                ui.label(
                    RichText::new(app.t("dialog-launch-wait"))
                        .size(12.5 * scale)
                        .color(WARNING),
                );
            }
        } else if let Some(outcome) = &state.outcome {
            ui.label(
                RichText::new(app.t("dialog-launch-sent"))
                    .size(23.0 * scale)
                    .strong()
                    .color(OK),
            );
            ui.label(
                RichText::new(&outcome.message)
                    .size(13.0 * scale)
                    .color(MUTED),
            );
            ui.label(
                RichText::new(app.t("dialog-launch-no-status"))
                    .size(12.0 * scale)
                    .color(MUTED),
            );
        }
        ui.add_space(12.0 * scale);
        ui.horizontal(|ui| {
            if ui
                .add_sized(
                    Vec2::new(200.0 * scale, 42.0 * scale),
                    secondary_button(app.t("dialog-launch-continue")),
                )
                .clicked()
            {
                dismiss = true;
            }
            if ui
                .add_sized(
                    Vec2::new(230.0 * scale, 42.0 * scale),
                    secondary_button(app.t("dialog-launch-minimize")),
                )
                .clicked()
            {
                minimize = true;
                dismiss = true;
            }
        });
    });
    if dismiss {
        app.launch = None;
    }
    if minimize {
        app.request_minimize(ctx);
    } else if dismiss {
        ctx.request_repaint();
    }
}

pub fn draw_fatal(app: &mut CouchDeckApp, ctx: &egui::Context) {
    let scale = ui::ui_scale(ctx);
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(ui::BACKGROUND))
        .show(ctx, |ui| {
            let screen = ctx.screen_rect();
            ui.painter()
                .rect_filled(screen, 0, Color32::from_rgb(12, 14, 20));
            let card = Rect::from_center_size(
                screen.center(),
                Vec2::new((900.0 * scale).min(screen.width() - 80.0), 470.0 * scale),
            );
            let painter = ui.painter().clone();
            painter.rect_filled(card, CornerRadius::same(20), CARD);
            painter.rect_stroke(
                card,
                CornerRadius::same(20),
                Stroke::new(2.0, DANGER),
                egui::StrokeKind::Inside,
            );
            painter.text(
                egui::pos2(card.center().x, card.top() + 48.0 * scale),
                Align2::CENTER_TOP,
                app.t("dialog-fatal-title"),
                FontId::proportional(28.0 * scale),
                TEXT,
            );
            if let Some(fatal) = &app.fatal {
                let mut message = fatal.message.clone();
                if let Some(backup) = &fatal.backup {
                    message.push_str(&format!(
                        "\n\n{}",
                        app.tf(
                            "dialog-fatal-backup",
                            &[("path", backup.display().to_string().into())]
                        )
                    ));
                }
                let message_rect = Rect::from_min_max(
                    egui::pos2(card.left() + 44.0 * scale, card.top() + 100.0 * scale),
                    egui::pos2(card.right() - 44.0 * scale, card.bottom() - 120.0 * scale),
                );
                ui.scope_builder(
                    egui::UiBuilder::new()
                        .max_rect(message_rect)
                        .id_salt("fatal-message"),
                    |ui| {
                        ui.add(
                            egui::Label::new(
                                RichText::new(message).size(13.5 * scale).color(MUTED),
                            )
                            .wrap(),
                        );
                    },
                );
            }
            let buttons = Rect::from_center_size(
                egui::pos2(card.center().x, card.bottom() - 62.0 * scale),
                Vec2::new(card.width() - 80.0 * scale, 52.0 * scale),
            );
            ui.scope_builder(
                egui::UiBuilder::new()
                    .max_rect(buttons)
                    .id_salt("fatal-actions"),
                |ui| {
                    ui.horizontal_centered(|ui| {
                        let focus = app.fatal.as_ref().map(|f| f.focus).unwrap_or(1);
                        let group_width = 180.0 * scale + 10.0 * scale + 340.0 * scale;
                        ui.add_space(((ui.available_width() - group_width) * 0.5).max(0.0));
                        let quit_response = ui.add_sized(
                            Vec2::new(180.0 * scale, 48.0 * scale),
                            secondary_button(app.t("dialog-fatal-quit")),
                        );
                        if quit_response.clicked() {
                            app.should_quit = true;
                        }
                        let reset_response = ui.add_sized(
                            Vec2::new(340.0 * scale, 48.0 * scale),
                            danger_button(app.t("dialog-fatal-reset")),
                        );
                        if reset_response.clicked() {
                            app.reset_after_fatal();
                            ui.ctx().request_repaint();
                        }
                        let focused_rect = if focus == 0 {
                            quit_response.rect
                        } else {
                            reset_response.rect
                        };
                        ui.painter().rect_stroke(
                            focused_rect.expand(3.0 * scale),
                            12.0,
                            Stroke::new(2.5 * scale, ACCENT),
                            egui::StrokeKind::Outside,
                        );
                    });
                },
            );
            painter.text(
                egui::pos2(card.center().x, card.bottom() - 18.0 * scale),
                Align2::CENTER_BOTTOM,
                app.t("dialog-fatal-hint"),
                FontId::proportional(11.5 * scale),
                MUTED,
            );
        });
}

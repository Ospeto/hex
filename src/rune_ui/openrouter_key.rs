//! Masked Labs entry; only a bounded background worker touches the OS vault.
use std::sync::mpsc::{Receiver, channel};

use rune::{El, TextEdit, col, row, text};
use rune_kit::{Button, ButtonVariant, TextInput};

use super::Ui;
use super::controls::{DESCRIPTION, Paint, PushSize};
use super::lists::{Tone, tone_tag};
use crate::desktop::i18n::tr;
use crate::transcription::openrouter::credentials::{self, Status};

enum Action {
    Check,
    Save(String),
    Remove,
}

pub(super) struct OpenRouterKeyState {
    draft: TextEdit,
    status: Status,
    error: Option<String>,
    worker: Option<Receiver<Result<Status, String>>>,
    show_missing: bool,
    confirm_remove: bool,
}

impl Default for OpenRouterKeyState {
    fn default() -> Self {
        Self {
            draft: TextEdit::new("").secure(),
            status: Status::Missing,
            error: None,
            worker: None,
            show_missing: false,
            confirm_remove: false,
        }
    }
}

impl OpenRouterKeyState {
    pub(super) fn clear_draft(&mut self) {
        self.draft.clear();
        self.show_missing = false;
        self.confirm_remove = false;
    }
}

fn open_drawer(ui: &mut Ui) {
    ui.labs_open = true;
    ui.lab_store.drawer = Some(super::labs::Lab::Feature(
        crate::labs::Feature::OpenRouterDictation,
    ));
    ui.picker.open = false;
    ui.open = None;
}

pub(super) fn refresh(ui: &mut Ui, show_missing: bool) {
    if !ui.live() {
        if show_missing && ui.openrouter_key.status == Status::Missing {
            open_drawer(ui);
        }
        return;
    }
    ui.openrouter_key.show_missing |= show_missing;
    if ui.openrouter_key.worker.is_none() {
        start(ui, Action::Check);
    }
}

fn start(ui: &mut Ui, action: Action) {
    if ui.openrouter_key.worker.is_some() {
        return;
    }
    if !ui.live() {
        ui.openrouter_key.status = match action {
            Action::Save(_) => Status::Saved,
            Action::Remove => Status::Missing,
            Action::Check => ui.openrouter_key.status,
        };
        return;
    }
    let (sender, receiver) = channel();
    let waker = ui.modes.waker.clone();
    let started = std::thread::Builder::new()
        .name("hex-openrouter-key".into())
        .spawn(move || {
            let result = match action {
                Action::Check => credentials::status(),
                Action::Save(key) => credentials::save(&key).map(|()| Status::Saved),
                Action::Remove => credentials::remove().map(|()| Status::Missing),
            };
            let _ = sender.send(result);
            if let Some(waker) = waker {
                waker();
            }
        });
    ui.openrouter_key.error = None;
    if started.is_ok() {
        ui.openrouter_key.worker = Some(receiver);
    } else {
        ui.openrouter_key.error = Some(tr("OpenRouter credential worker could not start").into());
        if ui.openrouter_key.show_missing {
            open_drawer(ui);
        }
    }
}

fn save(ui: &mut Ui) {
    if ui.openrouter_key.worker.is_some() {
        return;
    }
    let key = ui.openrouter_key.draft.text.clone();
    ui.openrouter_key.clear_draft();
    if !key.trim().is_empty() {
        start(ui, Action::Save(key));
    }
}

pub(super) fn poll(ui: &mut Ui) -> bool {
    // Navigating away also dismisses the editor: no draft survives a hidden drawer.
    if !ui.labs_open
        || ui.lab_store.drawer
            != Some(super::labs::Lab::Feature(
                crate::labs::Feature::OpenRouterDictation,
            ))
    {
        ui.openrouter_key.draft.clear();
        ui.openrouter_key.confirm_remove = false;
    }
    if ui
        .window
        .as_ref()
        .is_some_and(|window| !window.is_visible())
    {
        ui.openrouter_key.clear_draft();
    }
    let Some(worker) = &ui.openrouter_key.worker else {
        return false;
    };
    let result = match worker.try_recv() {
        Ok(result) => result,
        Err(std::sync::mpsc::TryRecvError::Empty) => return false,
        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
            Err(tr("OpenRouter credential worker disconnected").into())
        }
    };
    let show_missing = ui.openrouter_key.show_missing;
    ui.openrouter_key.worker = None;
    ui.openrouter_key.clear_draft();
    match result {
        Ok(status) => {
            ui.openrouter_key.status = status;
            ui.openrouter_key.error = None;
        }
        Err(error) => ui.openrouter_key.error = Some(error),
    }
    if show_missing
        && (ui.openrouter_key.status == Status::Missing || ui.openrouter_key.error.is_some())
    {
        open_drawer(ui);
    }
    true
}

pub(super) fn panel(ui: &Ui, p: Paint) -> El {
    let state = &ui.openrouter_key;
    let busy = state.worker.is_some();
    let (label, tone, note) = if state.error.is_some() {
        (tr("Error"), Tone::Danger, "")
    } else if busy {
        (tr("Working…"), Tone::Neutral, "")
    } else {
        match state.status {
            Status::Missing => (
                tr("Missing"),
                Tone::Neutral,
                tr("Save an API key to enable OpenRouter dictation"),
            ),
            Status::Saved => (
                tr("Saved"),
                Tone::Success,
                tr("API key is saved securely in your system credential store"),
            ),
            Status::Environment => (
                tr("Environment"),
                Tone::Neutral,
                tr("Using HEX_OPENROUTER_API_KEY from environment"),
            ),
        }
    };
    let input = TextInput::new(&state.draft)
        .secure()
        .label(tr("OpenRouter API key"))
        .placeholder(tr("Paste API key…"))
        .width(None)
        .disabled(busy)
        .on_edit(|ui: &mut Ui| &mut ui.openrouter_key.draft)
        .build(p.k)
        .key("openrouter-key-input");
    let save_label = if state.status == Status::Saved {
        tr("Replace key")
    } else {
        tr("Save key")
    };
    let mut actions = row().key("actions").gap(8.0).center_y().flex_wrap().child(
        Button::new(save_label)
            .variant(ButtonVariant::Contrast)
            .disabled(busy || state.draft.text.trim().is_empty())
            .on_press(save)
            .build_push(p.k)
            .key("openrouter-save-key"),
    );
    if state.confirm_remove {
        actions = actions
            .child(
                Button::new(tr("Confirm remove"))
                    .variant(ButtonVariant::Danger)
                    .disabled(busy)
                    .on_press(|ui: &mut Ui| {
                        ui.openrouter_key.clear_draft();
                        start(ui, Action::Remove);
                    })
                    .build_push(p.k)
                    .key("openrouter-confirm-remove"),
            )
            .child(
                Button::new(tr("Cancel"))
                    .variant(ButtonVariant::Neutral)
                    .on_press(|ui: &mut Ui| ui.openrouter_key.confirm_remove = false)
                    .build_push(p.k)
                    .key("openrouter-cancel-remove"),
            );
    } else if state.status != Status::Missing {
        actions = actions.child(
            Button::new(tr("Remove key…"))
                .variant(ButtonVariant::Neutral)
                .disabled(busy)
                .on_press(|ui: &mut Ui| ui.openrouter_key.confirm_remove = true)
                .build_push(p.k)
                .key("openrouter-remove-key"),
        );
    }
    if state.error.is_some() {
        actions = actions.child(
            Button::new(tr("Retry"))
                .variant(ButtonVariant::Neutral)
                .disabled(busy)
                .on_press(|ui: &mut Ui| refresh(ui, false))
                .build_push(p.k)
                .key("openrouter-retry-key"),
        );
    }
    col()
        .key("openrouter-key-panel")
        .padding(14.0)
        .gap(10.0)
        .radius(p.k.radius.md)
        .fill(p.k.c.layer_01)
        .border(p.hair, p.k.c.border_weak)
        .child(
            row()
                .key("header")
                .gap(8.0)
                .center_y()
                .justify_between()
                .child(
                    text(tr("OpenRouter API key"))
                        .key("title")
                        .font(p.k.ty.ui_medium)
                        .color(p.text()),
                )
                .child(tone_tag(p, "status", label, tone)),
        )
        .child(
            text(state.error.as_deref().unwrap_or(note))
                .key("note")
                .font(DESCRIPTION)
                .color(if state.error.is_some() {
                    p.danger()
                } else {
                    p.muted()
                })
                .wrap(),
        )
        .child(input)
        .child(actions)
}

/// Fixtures contain no secret and never access vault/environment.
pub(super) fn fixture(ui: &mut Ui, saved: bool) {
    ui.openrouter_key = OpenRouterKeyState {
        status: if saved {
            Status::Saved
        } else {
            Status::Missing
        },
        ..Default::default()
    };
    open_drawer(ui);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_settings::AppSettings;
    use crate::rune_ui::Service;

    #[test]
    fn draft_is_secure_redacted_and_cleared_on_save_and_dismissal() {
        let mut ui = Ui::with(AppSettings::default(), Service::Fixture(Vec::new()));
        fixture(&mut ui, false);
        ui.openrouter_key
            .draft
            .replace_text("synthetic-ui-sentinel");
        assert!(ui.openrouter_key.draft.is_secure());
        assert!(!format!("{:?}", ui.openrouter_key.draft).contains("synthetic-ui-sentinel"));
        save(&mut ui);
        assert!(ui.openrouter_key.status == Status::Saved);
        assert!(ui.openrouter_key.draft.text.is_empty());
        ui.openrouter_key
            .draft
            .replace_text("synthetic-ui-sentinel");
        super::super::labs::close_drawer(&mut ui);
        assert!(ui.openrouter_key.draft.text.is_empty());
    }

    #[test]
    fn missing_key_navigation_uses_existing_labs_drawer_without_reading_a_vault() {
        let mut ui = Ui::with(AppSettings::default(), Service::Fixture(Vec::new()));
        refresh(&mut ui, true);
        assert!(ui.labs_open);
        assert!(ui.openrouter_key.worker.is_none());
        assert_eq!(
            ui.lab_store.drawer,
            Some(super::super::labs::Lab::Feature(
                crate::labs::Feature::OpenRouterDictation
            ))
        );
    }
}

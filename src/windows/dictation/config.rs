//! The persisted settings projected into the Windows listener's startup config.
use super::*;

impl Shortcuts {
    pub(super) fn start(&self) -> Result<WindowsHotkeyMonitor> {
        // Double-tap-only depends on the double-tap window; without the lock
        // there is no second tap to wait for.
        WindowsHotkeyMonitor::start(
            self.hotkey.clone(),
            self.paste_last_hotkey.clone(),
            self.raw_dictation_hotkey.clone(),
            self.annotate_hotkey.clone(),
            self.cancel_hotkey.clone(),
            self.double_tap_lock,
            self.double_tap_lock && self.double_tap_only,
            self.shortcut_mode,
            self.share_dictation_shortcut,
        )
    }
}

impl WindowsDictationConfig {
    /// The listener configuration `settings` describe, with fresh runtime
    /// state; a host swaps in the state it shares with the listener.
    pub fn from_settings(settings: &crate::app_settings::AppSettings) -> Self {
        Self {
            device: settings.microphone.clone(),
            hotkey: WindowsHotkey::from(&settings.dictation_hotkey),
            paste_last_hotkey: settings.paste_last_hotkey.as_ref().map(WindowsHotkey::from),
            raw_dictation_hotkey: settings
                .raw_dictation_hotkey
                .as_ref()
                .map(WindowsHotkey::from),
            annotate_hotkey: settings
                .capture_on()
                .then(|| WindowsHotkey::from(&settings.capture.hotkey)),
            cancel_hotkey: settings.active_cancel_hotkey().map(WindowsHotkey::from),
            double_tap_lock: settings.double_tap_lock,
            double_tap_only: settings.double_tap_only,
            shortcut_mode: settings.shortcut_mode,
            share_dictation_shortcut: settings.share_dictation_shortcut,
            while_dictating: settings.recording_audio_behavior,
            release_microphone_while_idle: settings.release_microphone_while_idle,
            feedback_volume: if settings.sound_effects {
                (settings.sound_effect_volume.clamp(0.0, 1.0) * 100.0).round() as u8
            } else {
                0
            },
            sound_theme: settings.sound_theme,
            history: None,
            last_dictation: Arc::new(Mutex::new(None)),
            indicator: None,
            mode_runtime: Arc::new(RwLock::new(
                crate::dictation::modes::ModeRuntime::from_settings(settings),
            )),
            transformations: Arc::new(crate::personal_commands::TransformationClient::default()),
            fallback_to_default_device: true,
            paste_with_shift: settings.paste_with_shift,
            outcomes: None,
            commands: None,
        }
    }
}

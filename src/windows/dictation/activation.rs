//! Latest-wins model activation messages between the Windows host and listener.
use std::sync::mpsc;

use crate::app_settings::AppSettings;
use crate::transcription::TranscriptionDevice;
use crate::transcription::models::TranscriptionSelection;

#[derive(Clone, Debug)]
pub struct ListenerUpdate {
    pub settings: Box<AppSettings>,
    pub generation: u64,
    pub activation: Option<TranscriptionSelection>,
}

#[derive(Clone, Debug)]
pub struct ActivationOutcome {
    pub generation: u64,
    pub result: Result<(), String>,
}

pub(super) struct PendingReload {
    pub(super) generation: u64,
    pub(super) selection: TranscriptionSelection,
    pub(super) device: TranscriptionDevice,
    pub(super) receiver: mpsc::Receiver<Result<(), String>>,
}

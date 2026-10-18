//! Turn detection for agent CLI panels (phase 9): live activity and turn ends arrive on
//! the Channel of `turns_subscribe`.

use tauri::ipc::Channel;
use tauri::State;

use crate::error::AppResult;
use crate::turns::{TurnEvent, TurnWatcher};

/// Delivers `TurnEvent`s to `on_event` until the next subscribe (or a reload).
#[tauri::command]
pub async fn turns_subscribe(
    turns: State<'_, TurnWatcher>,
    on_event: Channel<TurnEvent>,
) -> AppResult<()> {
    turns.subscribe(Box::new(move |event| {
        let _ = on_event.send(event);
    }));
    Ok(())
}

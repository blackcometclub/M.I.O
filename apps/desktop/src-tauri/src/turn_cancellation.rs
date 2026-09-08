use crate::ai_dispatch::DesktopAiDispatcher;
use crate::room_orchestration::DesktopRoomOrchestrator;
use crate::room_source::DesktopRoomSource;
use moe_adapter_sdk::TextTurnCancellation;
use moe_core::RoomSource;
use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::State;

struct ActiveTurn {
    room_id: String,
    cancellation: TextTurnCancellation,
}

#[derive(Default)]
pub(crate) struct DesktopTurnCancellations {
    active: Mutex<BTreeMap<String, ActiveTurn>>,
}

impl DesktopTurnCancellations {
    pub(crate) fn register(
        self: &Arc<Self>,
        room_id: &str,
        dispatch_id: &str,
    ) -> Result<RegisteredTurn, ()> {
        let cancellation = TextTurnCancellation::default();
        let mut active = self.active.lock().map_err(|_| ())?;
        if active.contains_key(dispatch_id) {
            return Err(());
        }
        active.insert(
            dispatch_id.to_owned(),
            ActiveTurn {
                room_id: room_id.to_owned(),
                cancellation: cancellation.clone(),
            },
        );
        Ok(RegisteredTurn {
            owner: self.clone(),
            dispatch_id: dispatch_id.to_owned(),
            cancellation,
        })
    }

    pub(crate) fn cancel_room(&self, room_id: &str) -> usize {
        let Ok(active) = self.active.lock() else {
            return 0;
        };
        let mut cancelled = 0;
        for turn in active.values().filter(|turn| turn.room_id == room_id) {
            if !turn.cancellation.is_cancelled() {
                turn.cancellation.cancel();
                cancelled += 1;
            }
        }
        cancelled
    }

    fn remove(&self, dispatch_id: &str) {
        if let Ok(mut active) = self.active.lock() {
            active.remove(dispatch_id);
        }
    }
}

pub(crate) struct RegisteredTurn {
    owner: Arc<DesktopTurnCancellations>,
    dispatch_id: String,
    cancellation: TextTurnCancellation,
}

impl RegisteredTurn {
    pub(crate) fn cancellation(&self) -> TextTurnCancellation {
        self.cancellation.clone()
    }
}

impl Drop for RegisteredTurn {
    fn drop(&mut self) {
        self.owner.remove(&self.dispatch_id);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DesktopTurnCancellationSuccess {
    ok: bool,
    cancelled_turns: usize,
}

#[tauri::command]
pub(crate) async fn desktop_room_cancel_turn(
    source: State<'_, Arc<DesktopRoomSource>>,
    dispatcher: State<'_, Arc<DesktopAiDispatcher>>,
    orchestrator: State<'_, Arc<DesktopRoomOrchestrator>>,
    room_id: String,
    message_id: String,
) -> Result<DesktopTurnCancellationSuccess, &'static str> {
    source
        .find_message(&room_id, &message_id)
        .map_err(|_| "The Room message to stop is unavailable.")?;
    let dispatcher = dispatcher.inner().clone();
    let orchestrator = orchestrator.inner().clone();
    let room_id = room_id.clone();
    let cancelled_turns = tauri::async_runtime::spawn_blocking(move || {
        for attempt in 0..=20 {
            let cancelled =
                dispatcher.cancel_room_turns(&room_id) + orchestrator.cancel_room_turns(&room_id);
            if cancelled > 0 || attempt == 20 {
                return cancelled;
            }
            thread::sleep(Duration::from_millis(25));
        }
        0
    })
    .await
    .map_err(|_| "The stop request could not be completed.")?;
    Ok(DesktopTurnCancellationSuccess {
        ok: true,
        cancelled_turns,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancellation_is_room_scoped_and_removed_with_the_turn() {
        let registry = Arc::new(DesktopTurnCancellations::default());
        let room_one = registry.register("room-1", "dispatch-1").unwrap();
        let room_two = registry.register("room-2", "dispatch-2").unwrap();

        assert_eq!(registry.cancel_room("room-1"), 1);
        assert!(room_one.cancellation().is_cancelled());
        assert!(!room_two.cancellation().is_cancelled());

        drop(room_one);
        assert_eq!(registry.cancel_room("room-1"), 0);
    }
}

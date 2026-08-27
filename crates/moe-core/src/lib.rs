#![forbid(unsafe_code)]

mod conductor;
mod room;

pub use conductor::{
    CONDUCTOR_PLAN_V1_CAPABILITY, ConductorCapabilities, ConductorDelegation,
    ConductorDelegationIds, ConductorDelegationState, ConductorNextAction, ConductorOperation,
    ConductorOperationError, ConductorOperationIds, ConductorOperationStage, ConductorParticipant,
    ConductorPlanContext, ConductorPlanError, ConductorPlanMode, ConductorPlanV1, WorkerOutcome,
    parse_conductor_plan_v1,
};
pub use room::{
    InMemoryRoomSource, Room, RoomCatalogError, RoomCatalogSource, RoomCreateDraft,
    RoomCreateDraftError, RoomMessage, RoomMessageDraft, RoomMessageDraftError,
    RoomMessageFindError, RoomMessageProvenance, RoomMutationError, RoomMutationStatus,
    RoomMutationSuccess, RoomParticipant, RoomParticipantKind, RoomReadQuery, RoomReadQueryError,
    RoomReadResult, RoomSnapshot, RoomSnapshotError, RoomSource, RoomStore, RoomSummary,
    RoomWriteError, RoomWriteStatus, RoomWriteSuccess,
};

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapStatus {
    pub app_name: String,
    pub core_version: String,
    pub protocol_version: String,
}

pub fn bootstrap_status() -> BootstrapStatus {
    BootstrapStatus {
        app_name: "M.O.E.".into(),
        core_version: env!("CARGO_PKG_VERSION").into(),
        protocol_version: moe_protocol::PROTOCOL_VERSION.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::bootstrap_status;

    #[test]
    fn reports_named_foundation_versions() {
        let status = bootstrap_status();

        assert_eq!(status.app_name, "M.O.E.");
        assert!(!status.core_version.is_empty());
        assert!(!status.protocol_version.is_empty());
    }
}

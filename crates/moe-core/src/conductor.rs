use crate::RoomParticipantKind;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;

pub const CONDUCTOR_PLAN_V1_CAPABILITY: &str = "conductorPlanV1";

const MAXIMUM_IDENTIFIER_BYTES: usize = 128;
const MAXIMUM_PLAN_BYTES: usize = 16_000;
const MAXIMUM_DIRECT_ANSWER_BYTES: usize = 100_000;
const MAXIMUM_DELEGATION_TASK_BYTES: usize = 4_000;
const MAXIMUM_RESULT_TEXT_BYTES: usize = 100_000;
const MAXIMUM_WORKERS: usize = 3;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ConductorCapabilities {
    pub conductor_plan_v1: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConductorParticipant {
    pub id: String,
    pub kind: RoomParticipantKind,
    pub can_receive_delegation: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct ConductorPlanContext<'a> {
    pub owner_id: &'a str,
    pub conductor_id: &'a str,
    pub participants: &'a [ConductorParticipant],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ConductorPlanMode {
    Answer,
    Delegate,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConductorDelegation {
    pub target_participant_id: String,
    pub task: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConductorPlanV1 {
    version: u8,
    mode: ConductorPlanMode,
    direct_answer: Option<String>,
    delegations: Vec<ConductorDelegation>,
}

impl ConductorPlanV1 {
    pub fn version(&self) -> u8 {
        self.version
    }

    pub fn mode(&self) -> ConductorPlanMode {
        self.mode
    }

    pub fn direct_answer(&self) -> Option<&str> {
        self.direct_answer.as_deref()
    }

    pub fn delegations(&self) -> &[ConductorDelegation] {
        &self.delegations
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConductorPlanError {
    InvalidContext,
    InvalidUtf8,
    EnvelopeTooLarge,
    InvalidEnvelope,
    UnsupportedVersion,
    InvalidDirectAnswer,
    InvalidDelegationCount,
    InvalidDelegationTask,
    DuplicateTarget,
    TargetIsOwner,
    TargetIsConductor,
    TargetNotRoomMember,
    TargetIsHuman,
    TargetUnsupported,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RawConductorPlanV1 {
    version: u8,
    mode: ConductorPlanMode,
    direct_answer: Value,
    delegations: Vec<ConductorDelegation>,
}

pub fn parse_conductor_plan_v1(
    input: &[u8],
    context: &ConductorPlanContext<'_>,
) -> Result<ConductorPlanV1, ConductorPlanError> {
    validate_context(context)?;
    if input.len() > MAXIMUM_PLAN_BYTES {
        return Err(ConductorPlanError::EnvelopeTooLarge);
    }
    if std::str::from_utf8(input).is_err() {
        return Err(ConductorPlanError::InvalidUtf8);
    }
    let raw: RawConductorPlanV1 =
        serde_json::from_slice(input).map_err(|_| ConductorPlanError::InvalidEnvelope)?;
    if raw.version != 1 {
        return Err(ConductorPlanError::UnsupportedVersion);
    }

    let direct_answer = match raw.direct_answer {
        Value::Null => None,
        Value::String(answer) => Some(answer),
        _ => return Err(ConductorPlanError::InvalidDirectAnswer),
    };

    match raw.mode {
        ConductorPlanMode::Answer => {
            if raw.delegations.is_empty()
                && direct_answer
                    .as_deref()
                    .is_some_and(|answer| valid_bounded_text(answer, MAXIMUM_DIRECT_ANSWER_BYTES))
            {
                Ok(ConductorPlanV1 {
                    version: 1,
                    mode: raw.mode,
                    direct_answer,
                    delegations: Vec::new(),
                })
            } else {
                Err(ConductorPlanError::InvalidDirectAnswer)
            }
        }
        ConductorPlanMode::Delegate => {
            if direct_answer.is_some() || !(1..=MAXIMUM_WORKERS).contains(&raw.delegations.len()) {
                return Err(ConductorPlanError::InvalidDelegationCount);
            }
            validate_delegations(&raw.delegations, context)?;
            Ok(ConductorPlanV1 {
                version: 1,
                mode: raw.mode,
                direct_answer: None,
                delegations: raw.delegations,
            })
        }
    }
}

fn validate_context(context: &ConductorPlanContext<'_>) -> Result<(), ConductorPlanError> {
    if !valid_identifier(context.owner_id)
        || !valid_identifier(context.conductor_id)
        || context.owner_id == context.conductor_id
    {
        return Err(ConductorPlanError::InvalidContext);
    }
    let mut participant_ids = HashSet::new();
    if context.participants.iter().any(|participant| {
        !valid_identifier(&participant.id) || !participant_ids.insert(participant.id.as_str())
    }) {
        return Err(ConductorPlanError::InvalidContext);
    }
    let owner = context
        .participants
        .iter()
        .find(|participant| participant.id == context.owner_id);
    let conductor = context
        .participants
        .iter()
        .find(|participant| participant.id == context.conductor_id);
    if !owner.is_some_and(|participant| participant.kind == RoomParticipantKind::Human)
        || !conductor.is_some_and(|participant| participant.kind == RoomParticipantKind::Ai)
    {
        return Err(ConductorPlanError::InvalidContext);
    }
    Ok(())
}

fn validate_delegations(
    delegations: &[ConductorDelegation],
    context: &ConductorPlanContext<'_>,
) -> Result<(), ConductorPlanError> {
    let mut targets = HashSet::new();
    for delegation in delegations {
        if !valid_bounded_text(&delegation.task, MAXIMUM_DELEGATION_TASK_BYTES) {
            return Err(ConductorPlanError::InvalidDelegationTask);
        }
        if delegation.target_participant_id == context.owner_id {
            return Err(ConductorPlanError::TargetIsOwner);
        }
        if delegation.target_participant_id == context.conductor_id {
            return Err(ConductorPlanError::TargetIsConductor);
        }
        if !targets.insert(delegation.target_participant_id.as_str()) {
            return Err(ConductorPlanError::DuplicateTarget);
        }
        let Some(participant) = context
            .participants
            .iter()
            .find(|participant| participant.id == delegation.target_participant_id)
        else {
            return Err(ConductorPlanError::TargetNotRoomMember);
        };
        if participant.kind != RoomParticipantKind::Ai {
            return Err(ConductorPlanError::TargetIsHuman);
        }
        if !participant.can_receive_delegation {
            return Err(ConductorPlanError::TargetUnsupported);
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConductorOperationIds {
    pub operation_id: String,
    pub final_message_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConductorDelegationIds {
    pub message_id: String,
    pub dispatch_id: String,
}

impl ConductorOperationIds {
    pub fn derive(
        source_message_id: &str,
        conductor_id: &str,
    ) -> Result<Self, ConductorOperationError> {
        if !valid_identifier(source_message_id) || !valid_identifier(conductor_id) {
            return Err(ConductorOperationError::InvalidIdentity);
        }
        let operation_id =
            stable_identifier("conductor-operation-v1", &[source_message_id, conductor_id]);
        let final_message_id = stable_identifier("conductor-final-v1", &[operation_id.as_str()]);
        Ok(Self {
            operation_id,
            final_message_id,
        })
    }

    pub fn delegation(
        &self,
        target_participant_id: &str,
        ordinal: usize,
    ) -> Result<ConductorDelegationIds, ConductorOperationError> {
        if !valid_identifier(target_participant_id) || ordinal >= MAXIMUM_WORKERS {
            return Err(ConductorOperationError::InvalidIdentity);
        }
        let ordinal = ordinal.to_string();
        Ok(ConductorDelegationIds {
            message_id: stable_identifier(
                "conductor-delegation-v1",
                &[
                    self.operation_id.as_str(),
                    target_participant_id,
                    ordinal.as_str(),
                ],
            ),
            dispatch_id: stable_identifier(
                "conductor-dispatch-v1",
                &[
                    self.operation_id.as_str(),
                    target_participant_id,
                    ordinal.as_str(),
                ],
            ),
        })
    }
}

fn stable_identifier(prefix: &str, parts: &[&str]) -> String {
    let mut first = 0xcbf2_9ce4_8422_2325_u64;
    let mut second = 0x8422_2325_cbf2_9ce4_u64;
    for part in parts {
        for byte in (part.len() as u64).to_le_bytes() {
            fnv_step(&mut first, byte);
            fnv_step(&mut second, byte ^ 0xa5);
        }
        for &byte in part.as_bytes() {
            fnv_step(&mut first, byte);
            fnv_step(&mut second, byte ^ 0xa5);
        }
    }
    format!("{prefix}-{first:016x}{second:016x}")
}

fn fnv_step(state: &mut u64, byte: u8) {
    *state ^= u64::from(byte);
    *state = state.wrapping_mul(0x0000_0100_0000_01b3);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ConductorOperationStage {
    Prepared,
    Planning,
    Delegating,
    Synthesizing,
    Completed,
    Failed,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerOutcome {
    Succeeded {
        reply_message_id: String,
        body: String,
    },
    Failed {
        reason: String,
    },
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConductorDelegationState {
    pub delegation: ConductorDelegation,
    pub ids: ConductorDelegationIds,
    pub outcome: Option<WorkerOutcome>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConductorNextAction {
    BeginPlanning,
    RequestPlan {
        operation_id: String,
    },
    DispatchWorkers(Vec<ConductorDelegationState>),
    RequestSynthesis {
        operation_id: String,
        results: Vec<ConductorDelegationState>,
    },
    SaveFinalAnswer {
        message_id: String,
        body: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConductorOperationError {
    InvalidIdentity,
    InvalidStage,
    InvalidOutcome,
    ConflictingReplay,
    UnknownWorker,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConductorOperation {
    ids: ConductorOperationIds,
    source_message_id: String,
    conductor_id: String,
    stage: ConductorOperationStage,
    plan: Option<ConductorPlanV1>,
    delegations: Vec<ConductorDelegationState>,
    pending_final_answer: Option<String>,
}

impl ConductorOperation {
    pub fn try_new(
        source_message_id: String,
        conductor_id: String,
    ) -> Result<Self, ConductorOperationError> {
        let ids = ConductorOperationIds::derive(&source_message_id, &conductor_id)?;
        Ok(Self {
            ids,
            source_message_id,
            conductor_id,
            stage: ConductorOperationStage::Prepared,
            plan: None,
            delegations: Vec::new(),
            pending_final_answer: None,
        })
    }

    pub fn ids(&self) -> &ConductorOperationIds {
        &self.ids
    }

    pub fn source_message_id(&self) -> &str {
        &self.source_message_id
    }

    pub fn conductor_id(&self) -> &str {
        &self.conductor_id
    }

    pub fn stage(&self) -> ConductorOperationStage {
        self.stage
    }

    pub fn plan(&self) -> Option<&ConductorPlanV1> {
        self.plan.as_ref()
    }

    pub fn delegations(&self) -> &[ConductorDelegationState] {
        &self.delegations
    }

    pub fn pending_final_answer(&self) -> Option<&str> {
        self.pending_final_answer.as_deref()
    }

    pub fn next_action(&self) -> Option<ConductorNextAction> {
        match self.stage {
            ConductorOperationStage::Prepared => Some(ConductorNextAction::BeginPlanning),
            ConductorOperationStage::Planning => Some(ConductorNextAction::RequestPlan {
                operation_id: self.ids.operation_id.clone(),
            }),
            ConductorOperationStage::Delegating => {
                let pending = self
                    .delegations
                    .iter()
                    .filter(|delegation| delegation.outcome.is_none())
                    .cloned()
                    .collect::<Vec<_>>();
                (!pending.is_empty()).then_some(ConductorNextAction::DispatchWorkers(pending))
            }
            ConductorOperationStage::Synthesizing => {
                if let Some(body) = self.pending_final_answer.clone() {
                    Some(ConductorNextAction::SaveFinalAnswer {
                        message_id: self.ids.final_message_id.clone(),
                        body,
                    })
                } else {
                    Some(ConductorNextAction::RequestSynthesis {
                        operation_id: self.ids.operation_id.clone(),
                        results: self.delegations.clone(),
                    })
                }
            }
            ConductorOperationStage::Completed
            | ConductorOperationStage::Failed
            | ConductorOperationStage::Unknown => None,
        }
    }

    pub fn begin_planning(&mut self) -> Result<(), ConductorOperationError> {
        match self.stage {
            ConductorOperationStage::Prepared => {
                self.stage = ConductorOperationStage::Planning;
                Ok(())
            }
            ConductorOperationStage::Planning => Ok(()),
            _ => Err(ConductorOperationError::InvalidStage),
        }
    }

    pub fn accept_plan(&mut self, plan: ConductorPlanV1) -> Result<(), ConductorOperationError> {
        if let Some(existing) = self.plan.as_ref() {
            return if existing == &plan {
                Ok(())
            } else {
                Err(ConductorOperationError::ConflictingReplay)
            };
        }
        if self.stage != ConductorOperationStage::Planning {
            return Err(ConductorOperationError::InvalidStage);
        }

        match plan.mode {
            ConductorPlanMode::Answer => {
                let answer = plan
                    .direct_answer
                    .clone()
                    .filter(|answer| valid_bounded_text(answer, MAXIMUM_DIRECT_ANSWER_BYTES))
                    .ok_or(ConductorOperationError::InvalidOutcome)?;
                if !plan.delegations.is_empty() {
                    return Err(ConductorOperationError::InvalidOutcome);
                }
                self.pending_final_answer = Some(answer);
                self.stage = ConductorOperationStage::Synthesizing;
            }
            ConductorPlanMode::Delegate => {
                if plan.direct_answer.is_some()
                    || !(1..=MAXIMUM_WORKERS).contains(&plan.delegations.len())
                {
                    return Err(ConductorOperationError::InvalidOutcome);
                }
                let mut targets = HashSet::new();
                let mut states = Vec::with_capacity(plan.delegations.len());
                for (ordinal, delegation) in plan.delegations.iter().enumerate() {
                    if !valid_identifier(&delegation.target_participant_id)
                        || !valid_bounded_text(&delegation.task, MAXIMUM_DELEGATION_TASK_BYTES)
                        || !targets.insert(delegation.target_participant_id.as_str())
                    {
                        return Err(ConductorOperationError::InvalidOutcome);
                    }
                    states.push(ConductorDelegationState {
                        ids: self
                            .ids
                            .delegation(&delegation.target_participant_id, ordinal)?,
                        delegation: delegation.clone(),
                        outcome: None,
                    });
                }
                self.delegations = states;
                self.stage = ConductorOperationStage::Delegating;
            }
        }
        self.plan = Some(plan);
        Ok(())
    }

    pub fn record_worker_outcome(
        &mut self,
        target_participant_id: &str,
        outcome: WorkerOutcome,
    ) -> Result<(), ConductorOperationError> {
        let state = self
            .delegations
            .iter_mut()
            .find(|state| state.delegation.target_participant_id == target_participant_id)
            .ok_or(ConductorOperationError::UnknownWorker)?;
        if let Some(existing) = state.outcome.as_ref() {
            return if existing == &outcome {
                Ok(())
            } else {
                Err(ConductorOperationError::ConflictingReplay)
            };
        }
        if self.stage != ConductorOperationStage::Delegating || !valid_worker_outcome(&outcome) {
            return Err(ConductorOperationError::InvalidOutcome);
        }
        state.outcome = Some(outcome);
        if self
            .delegations
            .iter()
            .all(|delegation| delegation.outcome.is_some())
        {
            self.stage = ConductorOperationStage::Synthesizing;
        }
        Ok(())
    }

    pub fn record_synthesis(&mut self, answer: String) -> Result<(), ConductorOperationError> {
        if let Some(existing) = self.pending_final_answer.as_ref() {
            return if existing == &answer {
                Ok(())
            } else {
                Err(ConductorOperationError::ConflictingReplay)
            };
        }
        if self.stage != ConductorOperationStage::Synthesizing
            || self
                .plan
                .as_ref()
                .is_none_or(|plan| plan.mode != ConductorPlanMode::Delegate)
            || !valid_bounded_text(&answer, MAXIMUM_RESULT_TEXT_BYTES)
        {
            return Err(ConductorOperationError::InvalidOutcome);
        }
        self.pending_final_answer = Some(answer);
        Ok(())
    }

    pub fn mark_final_saved(&mut self) -> Result<(), ConductorOperationError> {
        match self.stage {
            ConductorOperationStage::Synthesizing if self.pending_final_answer.is_some() => {
                self.stage = ConductorOperationStage::Completed;
                Ok(())
            }
            ConductorOperationStage::Completed => Ok(()),
            _ => Err(ConductorOperationError::InvalidStage),
        }
    }

    pub fn mark_failed(&mut self) -> Result<(), ConductorOperationError> {
        if matches!(
            self.stage,
            ConductorOperationStage::Completed
                | ConductorOperationStage::Failed
                | ConductorOperationStage::Unknown
        ) {
            return if self.stage == ConductorOperationStage::Failed {
                Ok(())
            } else {
                Err(ConductorOperationError::InvalidStage)
            };
        }
        self.stage = ConductorOperationStage::Failed;
        Ok(())
    }

    pub fn mark_external_unknown(&mut self) -> Result<(), ConductorOperationError> {
        if matches!(
            self.stage,
            ConductorOperationStage::Completed
                | ConductorOperationStage::Failed
                | ConductorOperationStage::Unknown
        ) {
            return if self.stage == ConductorOperationStage::Unknown {
                Ok(())
            } else {
                Err(ConductorOperationError::InvalidStage)
            };
        }
        self.stage = ConductorOperationStage::Unknown;
        Ok(())
    }
}

fn valid_worker_outcome(outcome: &WorkerOutcome) -> bool {
    match outcome {
        WorkerOutcome::Succeeded {
            reply_message_id,
            body,
        } => {
            valid_identifier(reply_message_id)
                && valid_bounded_text(body, MAXIMUM_RESULT_TEXT_BYTES)
        }
        WorkerOutcome::Failed { reason } => {
            valid_bounded_text(reason, MAXIMUM_DELEGATION_TASK_BYTES)
        }
        WorkerOutcome::Unknown => true,
    }
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAXIMUM_IDENTIFIER_BYTES
        && value.bytes().enumerate().all(|(index, byte)| match byte {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' => true,
            b'.' | b'_' | b'-' => index > 0,
            _ => false,
        })
}

fn valid_bounded_text(value: &str, maximum_bytes: usize) -> bool {
    !value.trim().is_empty() && value.len() <= maximum_bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn participants() -> Vec<ConductorParticipant> {
        vec![
            ConductorParticipant {
                id: "owner".to_owned(),
                kind: RoomParticipantKind::Human,
                can_receive_delegation: false,
            },
            ConductorParticipant {
                id: "codex".to_owned(),
                kind: RoomParticipantKind::Ai,
                can_receive_delegation: true,
            },
            ConductorParticipant {
                id: "gemini".to_owned(),
                kind: RoomParticipantKind::Ai,
                can_receive_delegation: true,
            },
            ConductorParticipant {
                id: "claude".to_owned(),
                kind: RoomParticipantKind::Ai,
                can_receive_delegation: true,
            },
            ConductorParticipant {
                id: "grok".to_owned(),
                kind: RoomParticipantKind::Ai,
                can_receive_delegation: true,
            },
            ConductorParticipant {
                id: "future-ai".to_owned(),
                kind: RoomParticipantKind::Ai,
                can_receive_delegation: false,
            },
            ConductorParticipant {
                id: "guest".to_owned(),
                kind: RoomParticipantKind::Human,
                can_receive_delegation: false,
            },
        ]
    }

    fn context(participants: &[ConductorParticipant]) -> ConductorPlanContext<'_> {
        ConductorPlanContext {
            owner_id: "owner",
            conductor_id: "codex",
            participants,
        }
    }

    fn parse(input: &str) -> Result<ConductorPlanV1, ConductorPlanError> {
        let participants = participants();
        parse_conductor_plan_v1(input.as_bytes(), &context(&participants))
    }

    fn delegation_json(targets: &[&str]) -> String {
        let delegations = targets
            .iter()
            .map(|target| format!(r#"{{"targetParticipantId":"{target}","task":"task"}}"#))
            .collect::<Vec<_>>()
            .join(",");
        format!(
            r#"{{"version":1,"mode":"delegate","directAnswer":null,"delegations":[{delegations}]}}"#
        )
    }

    #[test]
    fn parses_a_strict_direct_answer() {
        let plan =
            parse(r#"{"version":1,"mode":"answer","directAnswer":"answer","delegations":[]}"#)
                .unwrap();
        assert_eq!(plan.mode, ConductorPlanMode::Answer);
        assert_eq!(plan.direct_answer.as_deref(), Some("answer"));

        assert_eq!(
            parse(
                r#"{"version":1,"mode":"answer","directAnswer":"answer","delegations":[],"extra":true}"#
            ),
            Err(ConductorPlanError::InvalidEnvelope)
        );
        assert_eq!(
            parse(r#"{"version":2,"mode":"answer","directAnswer":"answer","delegations":[]}"#),
            Err(ConductorPlanError::UnsupportedVersion)
        );
        assert_eq!(
            parse(r#"{"version":1,"mode":"answer","directAnswer":null,"delegations":[]}"#),
            Err(ConductorPlanError::InvalidDirectAnswer)
        );
    }

    #[test]
    fn accepts_one_to_three_unique_supported_workers() {
        for targets in [
            vec!["gemini"],
            vec!["gemini", "claude"],
            vec!["gemini", "claude", "grok"],
        ] {
            let plan = parse(&delegation_json(&targets)).unwrap();
            assert_eq!(plan.delegations.len(), targets.len());
        }
        assert_eq!(
            parse(&delegation_json(&["gemini", "claude", "grok", "future-ai"])),
            Err(ConductorPlanError::InvalidDelegationCount)
        );
    }

    #[test]
    fn rejects_unsafe_delegation_targets() {
        for (targets, expected) in [
            (
                vec!["gemini", "gemini"],
                ConductorPlanError::DuplicateTarget,
            ),
            (vec!["owner"], ConductorPlanError::TargetIsOwner),
            (vec!["codex"], ConductorPlanError::TargetIsConductor),
            (vec!["missing"], ConductorPlanError::TargetNotRoomMember),
            (vec!["guest"], ConductorPlanError::TargetIsHuman),
            (vec!["future-ai"], ConductorPlanError::TargetUnsupported),
        ] {
            assert_eq!(parse(&delegation_json(&targets)), Err(expected));
        }
    }

    #[test]
    fn rejects_malformed_unbounded_and_non_utf8_envelopes() {
        let participants = participants();
        let context = context(&participants);
        assert_eq!(
            parse_conductor_plan_v1(&[0xff], &context),
            Err(ConductorPlanError::InvalidUtf8)
        );
        assert_eq!(
            parse_conductor_plan_v1(&vec![b' '; MAXIMUM_PLAN_BYTES + 1], &context),
            Err(ConductorPlanError::EnvelopeTooLarge)
        );
        let too_long_task = "x".repeat(MAXIMUM_DELEGATION_TASK_BYTES + 1);
        let json = format!(
            r#"{{"version":1,"mode":"delegate","directAnswer":null,"delegations":[{{"targetParticipantId":"gemini","task":"{too_long_task}"}}]}}"#
        );
        assert_eq!(
            parse_conductor_plan_v1(json.as_bytes(), &context),
            Err(ConductorPlanError::InvalidDelegationTask)
        );
    }

    #[test]
    fn derives_stable_distinct_bounded_identifiers() {
        let first = ConductorOperationIds::derive("message-1", "codex").unwrap();
        let retry = ConductorOperationIds::derive("message-1", "codex").unwrap();
        let other = ConductorOperationIds::derive("message-2", "codex").unwrap();
        assert_eq!(first, retry);
        assert_ne!(first, other);
        assert!(first.operation_id.len() <= MAXIMUM_IDENTIFIER_BYTES);
        assert!(first.final_message_id.len() <= MAXIMUM_IDENTIFIER_BYTES);

        let gemini = first.delegation("gemini", 0).unwrap();
        let claude = first.delegation("claude", 1).unwrap();
        assert_ne!(gemini, claude);
        assert_eq!(gemini, first.delegation("gemini", 0).unwrap());
    }

    #[test]
    fn completes_a_direct_answer_idempotently() {
        let plan =
            parse(r#"{"version":1,"mode":"answer","directAnswer":"final","delegations":[]}"#)
                .unwrap();
        let mut operation =
            ConductorOperation::try_new("message-1".to_owned(), "codex".to_owned()).unwrap();
        assert_eq!(
            operation.next_action(),
            Some(ConductorNextAction::BeginPlanning)
        );
        operation.begin_planning().unwrap();
        operation.begin_planning().unwrap();
        operation.accept_plan(plan.clone()).unwrap();
        operation.accept_plan(plan).unwrap();
        assert!(matches!(
            operation.next_action(),
            Some(ConductorNextAction::SaveFinalAnswer { ref body, .. }) if body == "final"
        ));
        operation.mark_final_saved().unwrap();
        operation.mark_final_saved().unwrap();
        assert_eq!(operation.stage, ConductorOperationStage::Completed);
        assert_eq!(operation.next_action(), None);
    }

    struct FakeWorkerAdapter {
        outcomes: Vec<WorkerOutcome>,
    }

    impl FakeWorkerAdapter {
        fn resolve(&mut self, state: &ConductorDelegationState) -> WorkerOutcome {
            assert!(!state.ids.dispatch_id.is_empty());
            self.outcomes.remove(0)
        }
    }

    struct FakeConductorAdapter;

    impl FakeConductorAdapter {
        fn synthesize(&self, results: &[ConductorDelegationState]) -> String {
            assert_eq!(results.len(), 3);
            "integrated answer".to_owned()
        }
    }

    #[test]
    fn synthesizes_success_failure_and_unknown_without_retrying() {
        let plan = parse(&delegation_json(&["gemini", "claude", "grok"])).unwrap();
        let mut operation =
            ConductorOperation::try_new("message-1".to_owned(), "codex".to_owned()).unwrap();
        operation.begin_planning().unwrap();
        operation.accept_plan(plan).unwrap();

        let ConductorNextAction::DispatchWorkers(pending) = operation.next_action().unwrap() else {
            panic!("expected worker dispatches");
        };
        let mut workers = FakeWorkerAdapter {
            outcomes: vec![
                WorkerOutcome::Succeeded {
                    reply_message_id: "reply-1".to_owned(),
                    body: "result".to_owned(),
                },
                WorkerOutcome::Failed {
                    reason: "failed".to_owned(),
                },
                WorkerOutcome::Unknown,
            ],
        };
        for state in pending {
            let outcome = workers.resolve(&state);
            operation
                .record_worker_outcome(&state.delegation.target_participant_id, outcome.clone())
                .unwrap();
            operation
                .record_worker_outcome(&state.delegation.target_participant_id, outcome)
                .unwrap();
        }

        let ConductorNextAction::RequestSynthesis { results, .. } =
            operation.next_action().unwrap()
        else {
            panic!("expected synthesis");
        };
        assert!(matches!(results[2].outcome, Some(WorkerOutcome::Unknown)));
        let answer = FakeConductorAdapter.synthesize(&results);
        operation.record_synthesis(answer.clone()).unwrap();
        operation.record_synthesis(answer).unwrap();
        operation.mark_final_saved().unwrap();
        assert_eq!(operation.stage, ConductorOperationStage::Completed);
    }

    #[test]
    fn refuses_conflicting_reentry_and_stops_unknown_operations() {
        let direct =
            parse(r#"{"version":1,"mode":"answer","directAnswer":"first","delegations":[]}"#)
                .unwrap();
        let conflicting =
            parse(r#"{"version":1,"mode":"answer","directAnswer":"second","delegations":[]}"#)
                .unwrap();
        let mut operation =
            ConductorOperation::try_new("message-1".to_owned(), "codex".to_owned()).unwrap();
        operation.begin_planning().unwrap();
        operation.accept_plan(direct).unwrap();
        assert_eq!(
            operation.accept_plan(conflicting),
            Err(ConductorOperationError::ConflictingReplay)
        );

        let mut unknown =
            ConductorOperation::try_new("message-2".to_owned(), "codex".to_owned()).unwrap();
        unknown.begin_planning().unwrap();
        unknown.mark_external_unknown().unwrap();
        unknown.mark_external_unknown().unwrap();
        assert_eq!(unknown.stage, ConductorOperationStage::Unknown);
        assert_eq!(unknown.next_action(), None);
    }

    #[test]
    fn capability_is_explicit_and_disabled_by_default() {
        assert_eq!(CONDUCTOR_PLAN_V1_CAPABILITY, "conductorPlanV1");
        assert!(!ConductorCapabilities::default().conductor_plan_v1);
    }
}

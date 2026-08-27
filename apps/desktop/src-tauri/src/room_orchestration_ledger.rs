use moe_core::{ConductorOperationIds, ConductorOperationStage};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::env;
use std::error::Error;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

const FILE_VERSION: u8 = 1;
const FILE_NAME: &str = "room-orchestration-ledger-v1.json";
const MAXIMUM_FILE_BYTES: usize = 1024 * 1024;
const MAXIMUM_RECORDS: usize = 512;
const MAXIMUM_WORKERS: usize = 3;
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct OrchestrationDelegationLink {
    pub(crate) target_participant_id: String,
    pub(crate) message_id: String,
    pub(crate) dispatch_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RoomOrchestrationRecord {
    pub(crate) operation_id: String,
    pub(crate) room_id: String,
    pub(crate) source_message_id: String,
    pub(crate) conductor_id: String,
    pub(crate) conductor_session_id: Option<String>,
    pub(crate) final_message_id: String,
    pub(crate) stage: ConductorOperationStage,
    pub(crate) delegations: Vec<OrchestrationDelegationLink>,
    pub(crate) updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RoomOrchestrationBegin {
    Reserved(RoomOrchestrationRecord),
    Existing(RoomOrchestrationRecord),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RoomOrchestrationLedgerError {
    Invalid,
    Conflict,
    Io,
}

impl fmt::Display for RoomOrchestrationLedgerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Invalid => "the Room orchestration ledger is invalid",
            Self::Conflict => "the Room orchestration transition conflicts with durable state",
            Self::Io => "the Room orchestration ledger could not be accessed",
        })
    }
}

impl Error for RoomOrchestrationLedgerError {}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PersistedFile {
    file_version: u8,
    records: Vec<RoomOrchestrationRecord>,
}

struct LedgerPersistence {
    path: PathBuf,
}

impl LedgerPersistence {
    fn backup_path(&self) -> PathBuf {
        self.path.with_extension("json.backup")
    }

    fn load(
        &self,
    ) -> Result<(BTreeMap<String, RoomOrchestrationRecord>, bool), RoomOrchestrationLedgerError>
    {
        let backup = self.backup_path();
        let persisted = if self.path.is_file() {
            read_file(&self.path)?
        } else if !self.path.exists() && backup.is_file() {
            read_file(&backup)?
        } else if !self.path.exists() && !backup.exists() {
            return Ok((BTreeMap::new(), false));
        } else {
            return Err(RoomOrchestrationLedgerError::Invalid);
        };
        if persisted.file_version != FILE_VERSION || persisted.records.len() > MAXIMUM_RECORDS {
            return Err(RoomOrchestrationLedgerError::Invalid);
        }
        let mut records = BTreeMap::new();
        let mut normalized = false;
        for mut record in persisted.records {
            validate_record(&record)?;
            if matches!(
                record.stage,
                ConductorOperationStage::Planning | ConductorOperationStage::Synthesizing
            ) {
                record.stage = ConductorOperationStage::Unknown;
                normalized = true;
            }
            if records
                .insert(record.operation_id.clone(), record)
                .is_some()
            {
                return Err(RoomOrchestrationLedgerError::Invalid);
            }
        }
        Ok((records, normalized))
    }

    fn persist(
        &self,
        records: &BTreeMap<String, RoomOrchestrationRecord>,
    ) -> Result<(), RoomOrchestrationLedgerError> {
        if records.len() > MAXIMUM_RECORDS {
            return Err(RoomOrchestrationLedgerError::Invalid);
        }
        let parent = self
            .path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .ok_or(RoomOrchestrationLedgerError::Invalid)?;
        fs::create_dir_all(parent).map_err(|_| RoomOrchestrationLedgerError::Io)?;
        if !parent.is_dir() || (self.path.exists() && !self.path.is_file()) {
            return Err(RoomOrchestrationLedgerError::Invalid);
        }
        let body = serde_json::to_vec(&PersistedFile {
            file_version: FILE_VERSION,
            records: records.values().cloned().collect(),
        })
        .map_err(|_| RoomOrchestrationLedgerError::Invalid)?;
        if body.len() > MAXIMUM_FILE_BYTES {
            return Err(RoomOrchestrationLedgerError::Invalid);
        }

        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temp = parent.join(format!(
            ".{}.{}.{sequence}.tmp",
            self.path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or(RoomOrchestrationLedgerError::Invalid)?,
            std::process::id()
        ));
        let mut guard = TempGuard::new(temp.clone());
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|_| RoomOrchestrationLedgerError::Io)?;
        file.write_all(&body)
            .and_then(|_| file.sync_all())
            .map_err(|_| RoomOrchestrationLedgerError::Io)?;
        drop(file);

        let backup = self.backup_path();
        if backup.exists() && !backup.is_file() {
            return Err(RoomOrchestrationLedgerError::Invalid);
        }
        if self.path.exists() {
            if !self.path.is_file() {
                return Err(RoomOrchestrationLedgerError::Invalid);
            }
            if read_file(&self.path).is_ok() {
                if backup.exists() {
                    fs::remove_file(&backup).map_err(|_| RoomOrchestrationLedgerError::Io)?;
                }
                fs::rename(&self.path, &backup).map_err(|_| RoomOrchestrationLedgerError::Io)?;
            } else {
                return Err(RoomOrchestrationLedgerError::Invalid);
            }
        }
        if fs::rename(&temp, &self.path).is_err() {
            if !self.path.exists() && backup.is_file() {
                let _ = fs::rename(&backup, &self.path);
            }
            return Err(RoomOrchestrationLedgerError::Io);
        }
        guard.keep();
        Ok(())
    }
}

fn read_file(path: &Path) -> Result<PersistedFile, RoomOrchestrationLedgerError> {
    if !path.is_file() {
        return Err(RoomOrchestrationLedgerError::Invalid);
    }
    let file = File::open(path).map_err(|_| RoomOrchestrationLedgerError::Io)?;
    let size = file
        .metadata()
        .map_err(|_| RoomOrchestrationLedgerError::Io)?
        .len();
    if size == 0 || size > MAXIMUM_FILE_BYTES as u64 {
        return Err(RoomOrchestrationLedgerError::Invalid);
    }
    let mut body = Vec::with_capacity(size as usize);
    file.take(MAXIMUM_FILE_BYTES as u64 + 1)
        .read_to_end(&mut body)
        .map_err(|_| RoomOrchestrationLedgerError::Io)?;
    serde_json::from_slice(&body).map_err(|_| RoomOrchestrationLedgerError::Invalid)
}

struct TempGuard {
    path: PathBuf,
    keep: bool,
}

impl TempGuard {
    fn new(path: PathBuf) -> Self {
        Self { path, keep: false }
    }

    fn keep(&mut self) {
        self.keep = true;
    }
}

impl Drop for TempGuard {
    fn drop(&mut self) {
        if !self.keep {
            let _ = fs::remove_file(&self.path);
        }
    }
}

pub(crate) struct DesktopRoomOrchestrationLedger {
    records: Mutex<BTreeMap<String, RoomOrchestrationRecord>>,
    persistence: Option<LedgerPersistence>,
}

impl DesktopRoomOrchestrationLedger {
    #[cfg(test)]
    pub(crate) fn in_memory() -> Arc<Self> {
        Arc::new(Self {
            records: Mutex::new(BTreeMap::new()),
            persistence: None,
        })
    }

    fn persistent(path: PathBuf) -> Result<Arc<Self>, RoomOrchestrationLedgerError> {
        if !path.is_absolute() || path.file_name().is_none() {
            return Err(RoomOrchestrationLedgerError::Invalid);
        }
        let persistence = LedgerPersistence { path };
        let (records, normalized) = persistence.load()?;
        if normalized {
            persistence.persist(&records)?;
        }
        Ok(Arc::new(Self {
            records: Mutex::new(records),
            persistence: Some(persistence),
        }))
    }

    pub(crate) fn begin(
        &self,
        room_id: &str,
        source_message_id: &str,
        conductor_id: &str,
        updated_at: &str,
    ) -> Result<RoomOrchestrationBegin, RoomOrchestrationLedgerError> {
        let ids = ConductorOperationIds::derive(source_message_id, conductor_id)
            .map_err(|_| RoomOrchestrationLedgerError::Invalid)?;
        let record = RoomOrchestrationRecord {
            operation_id: ids.operation_id,
            room_id: room_id.to_owned(),
            source_message_id: source_message_id.to_owned(),
            conductor_id: conductor_id.to_owned(),
            conductor_session_id: None,
            final_message_id: ids.final_message_id,
            stage: ConductorOperationStage::Prepared,
            delegations: Vec::new(),
            updated_at: updated_at.to_owned(),
        };
        validate_record(&record)?;

        let mut records = self
            .records
            .lock()
            .map_err(|_| RoomOrchestrationLedgerError::Io)?;
        if let Some(existing) = records.get(&record.operation_id) {
            return if same_identity(existing, &record) {
                Ok(RoomOrchestrationBegin::Existing(existing.clone()))
            } else {
                Err(RoomOrchestrationLedgerError::Conflict)
            };
        }
        let mut next = records.clone();
        prune_terminal_record(&mut next);
        if next.len() >= MAXIMUM_RECORDS {
            return Err(RoomOrchestrationLedgerError::Invalid);
        }
        next.insert(record.operation_id.clone(), record.clone());
        self.persist(&next)?;
        *records = next;
        Ok(RoomOrchestrationBegin::Reserved(record))
    }

    pub(crate) fn mark_planning(
        &self,
        operation_id: &str,
        updated_at: &str,
    ) -> Result<(), RoomOrchestrationLedgerError> {
        self.transition(
            operation_id,
            &[ConductorOperationStage::Prepared],
            ConductorOperationStage::Planning,
            updated_at,
        )
    }

    pub(crate) fn mark_delegating(
        &self,
        operation_id: &str,
        conductor_session_id: &str,
        delegations: Vec<OrchestrationDelegationLink>,
        updated_at: &str,
    ) -> Result<(), RoomOrchestrationLedgerError> {
        if !valid_session_id(conductor_session_id)
            || !(1..=MAXIMUM_WORKERS).contains(&delegations.len())
        {
            return Err(RoomOrchestrationLedgerError::Invalid);
        }
        let mut records = self
            .records
            .lock()
            .map_err(|_| RoomOrchestrationLedgerError::Io)?;
        let current = records
            .get(operation_id)
            .ok_or(RoomOrchestrationLedgerError::Invalid)?;
        if current.stage == ConductorOperationStage::Delegating {
            return if current.conductor_session_id.as_deref() == Some(conductor_session_id)
                && current.delegations == delegations
            {
                Ok(())
            } else {
                Err(RoomOrchestrationLedgerError::Conflict)
            };
        }
        if current.stage != ConductorOperationStage::Planning || !valid_timestamp(updated_at) {
            return Err(RoomOrchestrationLedgerError::Conflict);
        }
        validate_delegations(current, &delegations)?;

        let mut next = records.clone();
        let record = next
            .get_mut(operation_id)
            .ok_or(RoomOrchestrationLedgerError::Invalid)?;
        record.conductor_session_id = Some(conductor_session_id.to_owned());
        record.delegations = delegations;
        record.stage = ConductorOperationStage::Delegating;
        record.updated_at = updated_at.to_owned();
        self.persist(&next)?;
        *records = next;
        Ok(())
    }

    pub(crate) fn mark_synthesizing(
        &self,
        operation_id: &str,
        updated_at: &str,
    ) -> Result<(), RoomOrchestrationLedgerError> {
        self.transition(
            operation_id,
            &[
                ConductorOperationStage::Planning,
                ConductorOperationStage::Delegating,
            ],
            ConductorOperationStage::Synthesizing,
            updated_at,
        )
    }

    pub(crate) fn mark_completed(
        &self,
        operation_id: &str,
        updated_at: &str,
    ) -> Result<(), RoomOrchestrationLedgerError> {
        self.transition(
            operation_id,
            &[ConductorOperationStage::Synthesizing],
            ConductorOperationStage::Completed,
            updated_at,
        )
    }

    pub(crate) fn mark_failed(
        &self,
        operation_id: &str,
        updated_at: &str,
    ) -> Result<(), RoomOrchestrationLedgerError> {
        self.transition(
            operation_id,
            &[
                ConductorOperationStage::Prepared,
                ConductorOperationStage::Planning,
                ConductorOperationStage::Delegating,
                ConductorOperationStage::Synthesizing,
            ],
            ConductorOperationStage::Failed,
            updated_at,
        )
    }

    pub(crate) fn mark_unknown(
        &self,
        operation_id: &str,
        updated_at: &str,
    ) -> Result<(), RoomOrchestrationLedgerError> {
        self.transition(
            operation_id,
            &[
                ConductorOperationStage::Planning,
                ConductorOperationStage::Synthesizing,
            ],
            ConductorOperationStage::Unknown,
            updated_at,
        )
    }

    #[cfg(test)]
    pub(crate) fn records_for_room(
        &self,
        room_id: &str,
    ) -> Result<Vec<RoomOrchestrationRecord>, RoomOrchestrationLedgerError> {
        if !valid_identifier(room_id) {
            return Err(RoomOrchestrationLedgerError::Invalid);
        }
        self.records
            .lock()
            .map_err(|_| RoomOrchestrationLedgerError::Io)
            .map(|records| {
                records
                    .values()
                    .filter(|record| record.room_id == room_id)
                    .cloned()
                    .collect()
            })
    }

    fn transition(
        &self,
        operation_id: &str,
        expected: &[ConductorOperationStage],
        next_stage: ConductorOperationStage,
        updated_at: &str,
    ) -> Result<(), RoomOrchestrationLedgerError> {
        if !valid_identifier(operation_id) || !valid_timestamp(updated_at) {
            return Err(RoomOrchestrationLedgerError::Invalid);
        }
        let mut records = self
            .records
            .lock()
            .map_err(|_| RoomOrchestrationLedgerError::Io)?;
        let current = records
            .get(operation_id)
            .ok_or(RoomOrchestrationLedgerError::Invalid)?;
        if current.stage == next_stage {
            return Ok(());
        }
        if !expected.contains(&current.stage) {
            return Err(RoomOrchestrationLedgerError::Conflict);
        }
        let mut next = records.clone();
        let record = next
            .get_mut(operation_id)
            .ok_or(RoomOrchestrationLedgerError::Invalid)?;
        record.stage = next_stage;
        record.updated_at = updated_at.to_owned();
        self.persist(&next)?;
        *records = next;
        Ok(())
    }

    fn persist(
        &self,
        records: &BTreeMap<String, RoomOrchestrationRecord>,
    ) -> Result<(), RoomOrchestrationLedgerError> {
        if let Some(persistence) = &self.persistence {
            persistence.persist(records)?;
        }
        Ok(())
    }
}

fn validate_record(record: &RoomOrchestrationRecord) -> Result<(), RoomOrchestrationLedgerError> {
    if !valid_identifier(&record.operation_id)
        || !valid_identifier(&record.room_id)
        || !valid_identifier(&record.source_message_id)
        || !valid_identifier(&record.conductor_id)
        || record
            .conductor_session_id
            .as_deref()
            .is_some_and(|session_id| !valid_session_id(session_id))
        || !valid_identifier(&record.final_message_id)
        || !valid_timestamp(&record.updated_at)
        || record.delegations.len() > MAXIMUM_WORKERS
    {
        return Err(RoomOrchestrationLedgerError::Invalid);
    }
    let ids = ConductorOperationIds::derive(&record.source_message_id, &record.conductor_id)
        .map_err(|_| RoomOrchestrationLedgerError::Invalid)?;
    if record.operation_id != ids.operation_id || record.final_message_id != ids.final_message_id {
        return Err(RoomOrchestrationLedgerError::Invalid);
    }
    validate_delegations(record, &record.delegations)
}

fn validate_delegations(
    record: &RoomOrchestrationRecord,
    delegations: &[OrchestrationDelegationLink],
) -> Result<(), RoomOrchestrationLedgerError> {
    let ids = ConductorOperationIds {
        operation_id: record.operation_id.clone(),
        final_message_id: record.final_message_id.clone(),
    };
    let mut targets = HashSet::new();
    for (ordinal, delegation) in delegations.iter().enumerate() {
        if !valid_identifier(&delegation.target_participant_id)
            || !targets.insert(delegation.target_participant_id.as_str())
        {
            return Err(RoomOrchestrationLedgerError::Invalid);
        }
        let expected = ids
            .delegation(&delegation.target_participant_id, ordinal)
            .map_err(|_| RoomOrchestrationLedgerError::Invalid)?;
        if delegation.message_id != expected.message_id
            || delegation.dispatch_id != expected.dispatch_id
        {
            return Err(RoomOrchestrationLedgerError::Invalid);
        }
    }
    Ok(())
}

fn same_identity(left: &RoomOrchestrationRecord, right: &RoomOrchestrationRecord) -> bool {
    left.operation_id == right.operation_id
        && left.room_id == right.room_id
        && left.source_message_id == right.source_message_id
        && left.conductor_id == right.conductor_id
        && left.final_message_id == right.final_message_id
}

fn prune_terminal_record(records: &mut BTreeMap<String, RoomOrchestrationRecord>) {
    if records.len() < MAXIMUM_RECORDS {
        return;
    }
    let oldest = records
        .values()
        .filter(|record| {
            matches!(
                record.stage,
                ConductorOperationStage::Completed | ConductorOperationStage::Failed
            )
        })
        .min_by(|left, right| {
            left.updated_at
                .cmp(&right.updated_at)
                .then_with(|| left.operation_id.cmp(&right.operation_id))
        })
        .map(|record| record.operation_id.clone());
    if let Some(operation_id) = oldest {
        records.remove(&operation_id);
    }
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().enumerate().all(|(index, byte)| match byte {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' => true,
            b'.' | b'_' | b'-' => index > 0,
            _ => false,
        })
}

fn valid_timestamp(value: &str) -> bool {
    !value.is_empty() && value.len() <= 64 && value.bytes().all(|byte| byte.is_ascii_graphic())
}

fn valid_session_id(value: &str) -> bool {
    !value.is_empty() && value.len() <= 256 && value.bytes().all(|byte| byte.is_ascii_graphic())
}

pub(crate) fn product_room_orchestration_ledger_file(app_data_dir: &Path) -> PathBuf {
    env::var_os("MOE_ROOM_ORCHESTRATION_LEDGER_FILE")
        .map(PathBuf::from)
        .unwrap_or_else(|| app_data_dir.join(FILE_NAME))
}

pub(crate) fn persistent_room_orchestration_ledger(
    path: PathBuf,
) -> Result<Arc<DesktopRoomOrchestrationLedger>, RoomOrchestrationLedgerError> {
    DesktopRoomOrchestrationLedger::persistent(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file(label: &str) -> PathBuf {
        env::temp_dir().join(format!(
            "mio-room-orchestration-{label}-{}-{}.json",
            std::process::id(),
            TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn operation_id(begin: &RoomOrchestrationBegin) -> &str {
        match begin {
            RoomOrchestrationBegin::Reserved(record) | RoomOrchestrationBegin::Existing(record) => {
                &record.operation_id
            }
        }
    }

    fn link(
        operation_id: &str,
        final_message_id: &str,
        target: &str,
        ordinal: usize,
    ) -> OrchestrationDelegationLink {
        let ids = ConductorOperationIds {
            operation_id: operation_id.to_owned(),
            final_message_id: final_message_id.to_owned(),
        }
        .delegation(target, ordinal)
        .unwrap();
        OrchestrationDelegationLink {
            target_participant_id: target.to_owned(),
            message_id: ids.message_id,
            dispatch_id: ids.dispatch_id,
        }
    }

    #[test]
    fn reserves_once_and_replays_the_same_operation() {
        let ledger = DesktopRoomOrchestrationLedger::in_memory();
        let first = ledger
            .begin("room-1", "message-1", "codex", "2026-08-14T00:00:00Z")
            .unwrap();
        let retry = ledger
            .begin("room-1", "message-1", "codex", "2026-08-14T00:00:01Z")
            .unwrap();
        assert!(matches!(first, RoomOrchestrationBegin::Reserved(_)));
        assert!(matches!(retry, RoomOrchestrationBegin::Existing(_)));
        assert_eq!(operation_id(&first), operation_id(&retry));
    }

    #[test]
    fn interrupted_external_conductor_turn_recovers_as_unknown() {
        let path = temp_file("unknown");
        let ledger = DesktopRoomOrchestrationLedger::persistent(path.clone()).unwrap();
        let begin = ledger
            .begin("room-1", "message-1", "codex", "2026-08-14T00:00:00Z")
            .unwrap();
        ledger
            .mark_planning(operation_id(&begin), "2026-08-14T00:00:01Z")
            .unwrap();
        drop(ledger);

        let reloaded = DesktopRoomOrchestrationLedger::persistent(path.clone()).unwrap();
        let records = reloaded.records_for_room("room-1").unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].stage, ConductorOperationStage::Unknown);
        assert_eq!(
            reloaded.mark_planning(&records[0].operation_id, "2026-08-14T00:00:02Z"),
            Err(RoomOrchestrationLedgerError::Conflict)
        );

        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_extension("json.backup"));
    }

    #[test]
    fn refuses_a_corrupt_primary_instead_of_regressing_to_an_older_stage() {
        let path = temp_file("corrupt-primary");
        let ledger = DesktopRoomOrchestrationLedger::persistent(path.clone()).unwrap();
        let begin = ledger
            .begin("room-1", "message-1", "codex", "2026-08-14T00:00:00Z")
            .unwrap();
        ledger
            .mark_planning(operation_id(&begin), "2026-08-14T00:00:01Z")
            .unwrap();
        drop(ledger);

        fs::write(&path, b"not json").unwrap();
        assert!(matches!(
            DesktopRoomOrchestrationLedger::persistent(path.clone()),
            Err(RoomOrchestrationLedgerError::Invalid)
        ));

        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_extension("json.backup"));
    }

    #[test]
    fn persists_verified_delegation_links_and_completion() {
        let path = temp_file("complete");
        let ledger = DesktopRoomOrchestrationLedger::persistent(path.clone()).unwrap();
        let begin = ledger
            .begin("room-1", "message-1", "codex", "2026-08-14T00:00:00Z")
            .unwrap();
        let record = match &begin {
            RoomOrchestrationBegin::Reserved(record) => record,
            RoomOrchestrationBegin::Existing(_) => panic!("expected reservation"),
        };
        let links = vec![
            link(&record.operation_id, &record.final_message_id, "gemini", 0),
            link(&record.operation_id, &record.final_message_id, "claude", 1),
        ];
        ledger
            .mark_planning(&record.operation_id, "2026-08-14T00:00:01Z")
            .unwrap();
        ledger
            .mark_delegating(
                &record.operation_id,
                "session-1",
                links.clone(),
                "2026-08-14T00:00:02Z",
            )
            .unwrap();
        ledger
            .mark_delegating(
                &record.operation_id,
                "session-1",
                links,
                "2026-08-14T00:00:03Z",
            )
            .unwrap();
        ledger
            .mark_synthesizing(&record.operation_id, "2026-08-14T00:00:04Z")
            .unwrap();
        ledger
            .mark_completed(&record.operation_id, "2026-08-14T00:00:05Z")
            .unwrap();
        drop(ledger);

        let reloaded = DesktopRoomOrchestrationLedger::persistent(path.clone()).unwrap();
        let records = reloaded.records_for_room("room-1").unwrap();
        assert_eq!(records[0].stage, ConductorOperationStage::Completed);
        assert_eq!(
            records[0].conductor_session_id.as_deref(),
            Some("session-1")
        );
        assert_eq!(records[0].delegations.len(), 2);

        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_extension("json.backup"));
    }

    #[test]
    fn rejects_forged_ids_and_out_of_order_transitions() {
        let ledger = DesktopRoomOrchestrationLedger::in_memory();
        let begin = ledger
            .begin("room-1", "message-1", "codex", "2026-08-14T00:00:00Z")
            .unwrap();
        let operation_id = operation_id(&begin);
        assert_eq!(
            ledger.mark_completed(operation_id, "2026-08-14T00:00:01Z"),
            Err(RoomOrchestrationLedgerError::Conflict)
        );
        ledger
            .mark_planning(operation_id, "2026-08-14T00:00:02Z")
            .unwrap();
        assert_eq!(
            ledger.mark_delegating(
                operation_id,
                "session-1",
                vec![OrchestrationDelegationLink {
                    target_participant_id: "gemini".to_owned(),
                    message_id: "forged-message".to_owned(),
                    dispatch_id: "forged-dispatch".to_owned(),
                }],
                "2026-08-14T00:00:03Z"
            ),
            Err(RoomOrchestrationLedgerError::Invalid)
        );
    }

    #[test]
    fn records_confirmed_failure_and_explicit_unknown_idempotently() {
        let failed = DesktopRoomOrchestrationLedger::in_memory();
        let begin = failed
            .begin("room-1", "message-1", "codex", "2026-08-14T00:00:00Z")
            .unwrap();
        failed
            .mark_failed(operation_id(&begin), "2026-08-14T00:00:01Z")
            .unwrap();
        failed
            .mark_failed(operation_id(&begin), "2026-08-14T00:00:02Z")
            .unwrap();

        let unknown = DesktopRoomOrchestrationLedger::in_memory();
        let begin = unknown
            .begin("room-1", "message-2", "codex", "2026-08-14T00:00:00Z")
            .unwrap();
        unknown
            .mark_planning(operation_id(&begin), "2026-08-14T00:00:01Z")
            .unwrap();
        unknown
            .mark_unknown(operation_id(&begin), "2026-08-14T00:00:02Z")
            .unwrap();
        unknown
            .mark_unknown(operation_id(&begin), "2026-08-14T00:00:03Z")
            .unwrap();
    }
}

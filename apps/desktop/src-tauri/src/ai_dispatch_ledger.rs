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

const DISPATCH_LEDGER_FILE_VERSION: u8 = 1;
const DISPATCH_LEDGER_FILE_NAME: &str = "ai-dispatch-ledger-v1.json";
const MAXIMUM_DISPATCH_LEDGER_BYTES: usize = 1024 * 1024;
const MAXIMUM_DISPATCH_RECORDS: usize = 512;
static TEMP_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum AiDispatchState {
    Prepared,
    ExternalStarted,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AiDispatchRecord {
    pub(crate) dispatch_id: String,
    pub(crate) room_id: String,
    pub(crate) source_message_id: String,
    pub(crate) recipient_id: String,
    pub(crate) reply_message_id: String,
    pub(crate) state: AiDispatchState,
    pub(crate) updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AiDispatchBegin {
    Reserved,
    Active,
    Existing(AiDispatchRecord),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AiDispatchLedgerError {
    Invalid,
    Io,
}

impl fmt::Display for AiDispatchLedgerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Invalid => "the AI dispatch ledger is invalid",
            Self::Io => "the AI dispatch ledger could not be accessed",
        })
    }
}

impl Error for AiDispatchLedgerError {}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PersistedDispatchRecord {
    dispatch_id: String,
    room_id: String,
    source_message_id: String,
    recipient_id: String,
    reply_message_id: String,
    state: AiDispatchState,
    updated_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PersistedDispatchLedger {
    file_version: u8,
    records: Vec<PersistedDispatchRecord>,
}

struct DispatchLedgerPersistence {
    path: PathBuf,
}

impl DispatchLedgerPersistence {
    fn backup_path(&self) -> PathBuf {
        self.path.with_extension("json.backup")
    }

    fn load(&self) -> Result<BTreeMap<String, AiDispatchRecord>, AiDispatchLedgerError> {
        let backup = self.backup_path();
        let source = if self.path.is_file() {
            &self.path
        } else if !self.path.exists() && backup.is_file() {
            &backup
        } else if !self.path.exists() && !backup.exists() {
            return Ok(BTreeMap::new());
        } else {
            return Err(AiDispatchLedgerError::Invalid);
        };
        let file = File::open(source).map_err(|_| AiDispatchLedgerError::Io)?;
        let size = file
            .metadata()
            .map_err(|_| AiDispatchLedgerError::Io)?
            .len();
        if size == 0 || size > MAXIMUM_DISPATCH_LEDGER_BYTES as u64 {
            return Err(AiDispatchLedgerError::Invalid);
        }
        let mut body = Vec::with_capacity(size as usize);
        file.take(MAXIMUM_DISPATCH_LEDGER_BYTES as u64 + 1)
            .read_to_end(&mut body)
            .map_err(|_| AiDispatchLedgerError::Io)?;
        let persisted: PersistedDispatchLedger =
            serde_json::from_slice(&body).map_err(|_| AiDispatchLedgerError::Invalid)?;
        if persisted.file_version != DISPATCH_LEDGER_FILE_VERSION
            || persisted.records.len() > MAXIMUM_DISPATCH_RECORDS
        {
            return Err(AiDispatchLedgerError::Invalid);
        }
        let mut records = BTreeMap::new();
        for persisted_record in persisted.records {
            let record = AiDispatchRecord {
                dispatch_id: persisted_record.dispatch_id,
                room_id: persisted_record.room_id,
                source_message_id: persisted_record.source_message_id,
                recipient_id: persisted_record.recipient_id,
                reply_message_id: persisted_record.reply_message_id,
                state: persisted_record.state,
                updated_at: persisted_record.updated_at,
            };
            validate_record(&record)?;
            if records.insert(record.dispatch_id.clone(), record).is_some() {
                return Err(AiDispatchLedgerError::Invalid);
            }
        }
        Ok(records)
    }

    fn persist(
        &self,
        records: &BTreeMap<String, AiDispatchRecord>,
    ) -> Result<(), AiDispatchLedgerError> {
        let parent = self
            .path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .ok_or(AiDispatchLedgerError::Invalid)?;
        fs::create_dir_all(parent).map_err(|_| AiDispatchLedgerError::Io)?;
        if !parent.is_dir() || (self.path.exists() && !self.path.is_file()) {
            return Err(AiDispatchLedgerError::Invalid);
        }
        let body = serde_json::to_vec(&PersistedDispatchLedger {
            file_version: DISPATCH_LEDGER_FILE_VERSION,
            records: records
                .values()
                .map(|record| PersistedDispatchRecord {
                    dispatch_id: record.dispatch_id.clone(),
                    room_id: record.room_id.clone(),
                    source_message_id: record.source_message_id.clone(),
                    recipient_id: record.recipient_id.clone(),
                    reply_message_id: record.reply_message_id.clone(),
                    state: record.state,
                    updated_at: record.updated_at.clone(),
                })
                .collect(),
        })
        .map_err(|_| AiDispatchLedgerError::Invalid)?;
        if body.len() > MAXIMUM_DISPATCH_LEDGER_BYTES {
            return Err(AiDispatchLedgerError::Invalid);
        }
        let sequence = TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temp = parent.join(format!(
            ".{}.{}.{sequence}.tmp",
            self.path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or(AiDispatchLedgerError::Invalid)?,
            std::process::id()
        ));
        let mut guard = TempGuard::new(temp.clone());
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|_| AiDispatchLedgerError::Io)?;
        file.write_all(&body)
            .and_then(|_| file.sync_all())
            .map_err(|_| AiDispatchLedgerError::Io)?;
        drop(file);
        let backup = self.backup_path();
        if backup.exists() {
            if !backup.is_file() {
                return Err(AiDispatchLedgerError::Invalid);
            }
            fs::remove_file(&backup).map_err(|_| AiDispatchLedgerError::Io)?;
        }
        if self.path.exists() {
            fs::rename(&self.path, &backup).map_err(|_| AiDispatchLedgerError::Io)?;
        }
        if fs::rename(&temp, &self.path).is_err() {
            if !self.path.exists() && backup.is_file() {
                let _ = fs::rename(&backup, &self.path);
            }
            return Err(AiDispatchLedgerError::Io);
        }
        guard.keep();
        Ok(())
    }
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

struct DispatchLedgerData {
    records: BTreeMap<String, AiDispatchRecord>,
    active: HashSet<String>,
}

pub(crate) struct DesktopAiDispatchLedger {
    data: Mutex<DispatchLedgerData>,
    persistence: Option<DispatchLedgerPersistence>,
}

impl DesktopAiDispatchLedger {
    #[cfg(test)]
    pub(crate) fn in_memory() -> Arc<Self> {
        Arc::new(Self {
            data: Mutex::new(DispatchLedgerData {
                records: BTreeMap::new(),
                active: HashSet::new(),
            }),
            persistence: None,
        })
    }

    fn persistent(path: PathBuf) -> Result<Arc<Self>, AiDispatchLedgerError> {
        let persistence = DispatchLedgerPersistence { path };
        let records = persistence.load()?;
        Ok(Arc::new(Self {
            data: Mutex::new(DispatchLedgerData {
                records,
                active: HashSet::new(),
            }),
            persistence: Some(persistence),
        }))
    }

    pub(crate) fn begin(
        &self,
        mut record: AiDispatchRecord,
    ) -> Result<AiDispatchBegin, AiDispatchLedgerError> {
        validate_record(&record)?;
        record.state = AiDispatchState::Prepared;
        let mut data = self.data.lock().map_err(|_| AiDispatchLedgerError::Io)?;
        if data.active.contains(&record.dispatch_id) {
            return Ok(AiDispatchBegin::Active);
        }
        if let Some(existing) = data.records.get(&record.dispatch_id) {
            if !same_dispatch(existing, &record) {
                return Err(AiDispatchLedgerError::Invalid);
            }
            return Ok(AiDispatchBegin::Existing(existing.clone()));
        }
        prune_terminal_records(&mut data.records);
        if data.records.len() >= MAXIMUM_DISPATCH_RECORDS {
            return Err(AiDispatchLedgerError::Invalid);
        }
        let mut next = data.records.clone();
        next.insert(record.dispatch_id.clone(), record.clone());
        self.persist(&next)?;
        data.records = next;
        data.active.insert(record.dispatch_id);
        Ok(AiDispatchBegin::Reserved)
    }

    pub(crate) fn mark_external_started(
        &self,
        dispatch_id: &str,
        updated_at: &str,
    ) -> Result<(), AiDispatchLedgerError> {
        self.transition(
            dispatch_id,
            AiDispatchState::Prepared,
            AiDispatchState::ExternalStarted,
            updated_at,
            false,
        )
    }

    pub(crate) fn mark_failed(
        &self,
        dispatch_id: &str,
        updated_at: &str,
    ) -> Result<(), AiDispatchLedgerError> {
        self.transition(
            dispatch_id,
            AiDispatchState::Prepared,
            AiDispatchState::Failed,
            updated_at,
            true,
        )
    }

    pub(crate) fn mark_external_preflight_failed(
        &self,
        dispatch_id: &str,
        updated_at: &str,
    ) -> Result<(), AiDispatchLedgerError> {
        self.transition(
            dispatch_id,
            AiDispatchState::ExternalStarted,
            AiDispatchState::Failed,
            updated_at,
            true,
        )
    }

    pub(crate) fn mark_completed(
        &self,
        dispatch_id: &str,
        updated_at: &str,
    ) -> Result<(), AiDispatchLedgerError> {
        self.transition(
            dispatch_id,
            AiDispatchState::ExternalStarted,
            AiDispatchState::Completed,
            updated_at,
            true,
        )
    }

    pub(crate) fn finish_unknown(&self, dispatch_id: &str) {
        if let Ok(mut data) = self.data.lock() {
            data.active.remove(dispatch_id);
        }
    }

    pub(crate) fn unresolved_for_room(
        &self,
        room_id: &str,
    ) -> Result<Vec<AiDispatchRecord>, AiDispatchLedgerError> {
        if !valid_entity_id(room_id) {
            return Err(AiDispatchLedgerError::Invalid);
        }
        let data = self.data.lock().map_err(|_| AiDispatchLedgerError::Io)?;
        Ok(data
            .records
            .values()
            .filter(|record| {
                record.room_id == room_id
                    && matches!(
                        record.state,
                        AiDispatchState::ExternalStarted | AiDispatchState::Completed
                    )
            })
            .cloned()
            .collect())
    }

    fn transition(
        &self,
        dispatch_id: &str,
        expected: AiDispatchState,
        next_state: AiDispatchState,
        updated_at: &str,
        finish: bool,
    ) -> Result<(), AiDispatchLedgerError> {
        if !valid_dispatch_id(dispatch_id) || !valid_timestamp(updated_at) {
            return Err(AiDispatchLedgerError::Invalid);
        }
        let mut data = self.data.lock().map_err(|_| AiDispatchLedgerError::Io)?;
        if !data.active.contains(dispatch_id) {
            return Err(AiDispatchLedgerError::Invalid);
        }
        let current = data
            .records
            .get(dispatch_id)
            .ok_or(AiDispatchLedgerError::Invalid)?;
        if current.state != expected {
            return Err(AiDispatchLedgerError::Invalid);
        }
        let mut next = data.records.clone();
        let record = next
            .get_mut(dispatch_id)
            .ok_or(AiDispatchLedgerError::Invalid)?;
        record.state = next_state;
        record.updated_at = updated_at.to_owned();
        self.persist(&next)?;
        data.records = next;
        if finish {
            data.active.remove(dispatch_id);
        }
        Ok(())
    }

    fn persist(
        &self,
        records: &BTreeMap<String, AiDispatchRecord>,
    ) -> Result<(), AiDispatchLedgerError> {
        if let Some(persistence) = &self.persistence {
            persistence.persist(records)?;
        }
        Ok(())
    }
}

pub(crate) fn product_ai_dispatch_ledger_file(app_data_dir: &Path) -> PathBuf {
    env::var_os("MOE_AI_DISPATCH_LEDGER_FILE")
        .map(PathBuf::from)
        .unwrap_or_else(|| app_data_dir.join(DISPATCH_LEDGER_FILE_NAME))
}

pub(crate) fn persistent_ai_dispatch_ledger(
    path: PathBuf,
) -> Result<Arc<DesktopAiDispatchLedger>, AiDispatchLedgerError> {
    DesktopAiDispatchLedger::persistent(path)
}

fn same_dispatch(left: &AiDispatchRecord, right: &AiDispatchRecord) -> bool {
    left.dispatch_id == right.dispatch_id
        && left.room_id == right.room_id
        && left.source_message_id == right.source_message_id
        && left.recipient_id == right.recipient_id
        && left.reply_message_id == right.reply_message_id
}

fn prune_terminal_records(records: &mut BTreeMap<String, AiDispatchRecord>) {
    if records.len() < MAXIMUM_DISPATCH_RECORDS {
        return;
    }
    let oldest = records
        .values()
        .filter(|record| {
            matches!(
                record.state,
                AiDispatchState::Completed | AiDispatchState::Failed
            )
        })
        .min_by(|left, right| {
            left.updated_at
                .cmp(&right.updated_at)
                .then_with(|| left.dispatch_id.cmp(&right.dispatch_id))
        })
        .map(|record| record.dispatch_id.clone());
    if let Some(dispatch_id) = oldest {
        records.remove(&dispatch_id);
    }
}

fn validate_record(record: &AiDispatchRecord) -> Result<(), AiDispatchLedgerError> {
    if !valid_dispatch_id(&record.dispatch_id)
        || !valid_entity_id(&record.room_id)
        || !valid_entity_id(&record.source_message_id)
        || !valid_entity_id(&record.recipient_id)
        || !valid_entity_id(&record.reply_message_id)
        || !valid_timestamp(&record.updated_at)
        || record.dispatch_id
            != format!(
                "room-message:{}:{}",
                record.source_message_id, record.recipient_id
            )
    {
        return Err(AiDispatchLedgerError::Invalid);
    }
    Ok(())
}

fn valid_dispatch_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 384
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':'))
}

fn valid_entity_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn valid_timestamp(value: &str) -> bool {
    !value.is_empty() && value.len() <= 64 && value.bytes().all(|byte| byte.is_ascii_graphic())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file(label: &str) -> PathBuf {
        env::temp_dir().join(format!(
            "moe-ai-dispatch-ledger-{label}-{}-{}.json",
            std::process::id(),
            TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn record(source_message_id: &str) -> AiDispatchRecord {
        AiDispatchRecord {
            dispatch_id: format!("room-message:{source_message_id}:codex"),
            room_id: "room-1".to_owned(),
            source_message_id: source_message_id.to_owned(),
            recipient_id: "codex".to_owned(),
            reply_message_id: format!("ai-reply-{source_message_id}-codex"),
            state: AiDispatchState::Prepared,
            updated_at: "2026-08-13T00:00:00Z".to_owned(),
        }
    }

    #[test]
    fn persists_external_started_as_unknown_after_restart() {
        let path = temp_file("unknown");
        let ledger = DesktopAiDispatchLedger::persistent(path.clone()).unwrap();
        assert_eq!(
            ledger.begin(record("message-1")).unwrap(),
            AiDispatchBegin::Reserved
        );
        ledger
            .mark_external_started("room-message:message-1:codex", "2026-08-13T00:00:01Z")
            .unwrap();
        drop(ledger);

        let reloaded = DesktopAiDispatchLedger::persistent(path.clone()).unwrap();
        let existing = reloaded.begin(record("message-1")).unwrap();
        assert!(matches!(
            existing,
            AiDispatchBegin::Existing(AiDispatchRecord {
                state: AiDispatchState::ExternalStarted,
                ..
            })
        ));

        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_extension("json.backup"));
    }

    #[test]
    fn persists_completed_and_failed_terminal_states() {
        let path = temp_file("terminal");
        let ledger = DesktopAiDispatchLedger::persistent(path.clone()).unwrap();
        ledger.begin(record("completed")).unwrap();
        ledger
            .mark_external_started("room-message:completed:codex", "2026-08-13T00:00:01Z")
            .unwrap();
        ledger
            .mark_completed("room-message:completed:codex", "2026-08-13T00:00:02Z")
            .unwrap();
        ledger.begin(record("failed")).unwrap();
        ledger
            .mark_failed("room-message:failed:codex", "2026-08-13T00:00:03Z")
            .unwrap();
        ledger.begin(record("preflight-failed")).unwrap();
        ledger
            .mark_external_started(
                "room-message:preflight-failed:codex",
                "2026-08-13T00:00:04Z",
            )
            .unwrap();
        ledger
            .mark_external_preflight_failed(
                "room-message:preflight-failed:codex",
                "2026-08-13T00:00:05Z",
            )
            .unwrap();
        drop(ledger);

        let reloaded = DesktopAiDispatchLedger::persistent(path.clone()).unwrap();
        assert!(matches!(
            reloaded.begin(record("completed")).unwrap(),
            AiDispatchBegin::Existing(AiDispatchRecord {
                state: AiDispatchState::Completed,
                ..
            })
        ));
        assert!(matches!(
            reloaded.begin(record("failed")).unwrap(),
            AiDispatchBegin::Existing(AiDispatchRecord {
                state: AiDispatchState::Failed,
                ..
            })
        ));
        assert!(matches!(
            reloaded.begin(record("preflight-failed")).unwrap(),
            AiDispatchBegin::Existing(AiDispatchRecord {
                state: AiDispatchState::Failed,
                ..
            })
        ));

        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_extension("json.backup"));
    }
}

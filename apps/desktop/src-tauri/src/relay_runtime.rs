use moe_relay_client::{
    RelayAccountId, RelayConnectionErrorCode, RelayLifecycle, RelayLifecycleAction,
    RelayLifecycleTransitionError, RelayRuntimePhase, RelayRuntimeStatus,
};
use std::{
    collections::HashMap,
    sync::{
        Arc, Condvar, Mutex, MutexGuard,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

type OwnedTask = Box<dyn Send>;
pub type DesktopRelayConnectionTask =
    Box<dyn FnOnce(&mut DesktopRelayTaskContext) -> RelayConnectionErrorCode + Send>;

pub type DesktopRelayConnectionTaskFactory =
    Arc<dyn Fn() -> DesktopRelayConnectionTask + Send + Sync>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopRelayRuntimeError {
    AlreadyActive,
    WorkerUnavailable,
    ConnectionTaskRequired,
    UnexpectedConnectionTask,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(not(test), allow(dead_code))]
pub enum DesktopRelayRuntimeEventKind {
    Connected,
    ConnectionFailed(RelayConnectionErrorCode),
    UnexpectedDisconnect(RelayConnectionErrorCode),
    RetryElapsed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopRelayRuntimeEvent {
    account_id: String,
    generation: u64,
    kind: DesktopRelayRuntimeEventKind,
}

#[cfg_attr(not(test), allow(dead_code))]
impl DesktopRelayRuntimeEvent {
    pub fn account_id(&self) -> &str {
        &self.account_id
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn kind(&self) -> DesktopRelayRuntimeEventKind {
        self.kind
    }

    fn is_terminal(&self) -> bool {
        !matches!(self.kind, DesktopRelayRuntimeEventKind::Connected)
    }
}

#[derive(Default)]
struct CancellationValue {
    cancelled: bool,
    hooks: Vec<Box<dyn FnOnce() + Send>>,
}

#[derive(Default)]
struct CancellationState {
    value: Mutex<CancellationValue>,
    wake: Condvar,
}

#[derive(Clone, Default)]
pub struct DesktopRelayCancellation {
    state: Arc<CancellationState>,
}

impl DesktopRelayCancellation {
    pub fn is_cancelled(&self) -> bool {
        self.state
            .value
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .cancelled
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn wait_cancelled(&self) {
        let value = self
            .state
            .value
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        drop(
            self.state
                .wake
                .wait_while(value, |value| !value.cancelled)
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
        );
    }

    pub fn on_cancel(&self, hook: impl FnOnce() + Send + 'static) {
        let mut value = self
            .state
            .value
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if value.cancelled {
            drop(value);
            hook();
        } else {
            value.hooks.push(Box::new(hook));
        }
    }

    fn wait_timeout(&self, duration: Duration) -> bool {
        let value = self
            .state
            .value
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let (value, _) = self
            .state
            .wake
            .wait_timeout_while(value, duration, |value| !value.cancelled)
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        value.cancelled
    }

    fn cancel(&self) {
        let hooks = {
            let mut value = self
                .state
                .value
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if value.cancelled {
                return;
            }
            value.cancelled = true;
            std::mem::take(&mut value.hooks)
        };
        self.state.wake.notify_all();
        for hook in hooks {
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(hook));
        }
    }
}

pub struct DesktopRelayTaskContext {
    account_id: String,
    generation: u64,
    cancellation: DesktopRelayCancellation,
    event_sender: Sender<DesktopRelayRuntimeEvent>,
    connected: bool,
}

#[cfg_attr(not(test), allow(dead_code))]
impl DesktopRelayTaskContext {
    pub fn cancellation(&self) -> &DesktopRelayCancellation {
        &self.cancellation
    }

    pub fn report_connected(&mut self) {
        if self.connected || self.cancellation.is_cancelled() {
            return;
        }
        self.connected = true;
        self.send(DesktopRelayRuntimeEventKind::Connected);
    }

    fn send(&self, kind: DesktopRelayRuntimeEventKind) {
        let _ = self.event_sender.send(DesktopRelayRuntimeEvent {
            account_id: self.account_id.clone(),
            generation: self.generation,
            kind,
        });
    }
}

struct DesktopRelayTaskHandle {
    cancellation: DesktopRelayCancellation,
    worker: Option<JoinHandle<()>>,
}

impl DesktopRelayTaskHandle {
    fn spawn<F>(generation: u64, task: F) -> Result<Self, DesktopRelayRuntimeError>
    where
        F: FnOnce(DesktopRelayCancellation) + Send + 'static,
    {
        let cancellation = DesktopRelayCancellation::default();
        let task_cancellation = cancellation.clone();
        let worker = thread::Builder::new()
            .name(format!("moe-relay-task-{generation}"))
            .spawn(move || task(task_cancellation))
            .map_err(|_| DesktopRelayRuntimeError::WorkerUnavailable)?;
        Ok(Self {
            cancellation,
            worker: Some(worker),
        })
    }
}

impl Drop for DesktopRelayTaskHandle {
    fn drop(&mut self) {
        self.cancellation.cancel();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

struct OwnedTaskEntry {
    generation: u64,
    _task: OwnedTask,
}

#[derive(Default)]
struct DesktopRelayTaskOwner {
    tasks: Mutex<HashMap<String, OwnedTaskEntry>>,
}

impl DesktopRelayTaskOwner {
    fn insert_with<C: Send + 'static>(
        &self,
        account_id: &RelayAccountId,
        generation: u64,
        create: impl FnOnce() -> Result<C, DesktopRelayRuntimeError>,
    ) -> Result<(), DesktopRelayRuntimeError> {
        let mut tasks = self.lock_tasks();
        if tasks.contains_key(account_id.as_str()) {
            return Err(DesktopRelayRuntimeError::AlreadyActive);
        }
        tasks.insert(
            account_id.as_str().to_owned(),
            OwnedTaskEntry {
                generation,
                _task: Box::new(create()?),
            },
        );
        Ok(())
    }

    fn stop(&self, account_id: &RelayAccountId) -> bool {
        self.stop_by_name(account_id.as_str())
    }

    fn stop_by_name(&self, account_id: &str) -> bool {
        let entry = self.lock_tasks().remove(account_id);
        let found = entry.is_some();
        drop(entry);
        found
    }

    fn stop_if_generation(&self, account_id: &str, generation: u64) -> bool {
        let mut tasks = self.lock_tasks();
        if tasks.get(account_id).map(|entry| entry.generation) != Some(generation) {
            return false;
        }
        let entry = tasks.remove(account_id);
        drop(tasks);
        drop(entry);
        true
    }

    fn is_current(&self, account_id: &str, generation: u64) -> bool {
        self.lock_tasks()
            .get(account_id)
            .is_some_and(|entry| entry.generation == generation)
    }

    fn shutdown(&self) {
        let tasks = std::mem::take(&mut *self.lock_tasks());
        drop(tasks);
    }

    #[cfg(test)]
    fn active_count(&self) -> usize {
        self.lock_tasks().len()
    }

    fn lock_tasks(&self) -> MutexGuard<'_, HashMap<String, OwnedTaskEntry>> {
        self.tasks
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl Drop for DesktopRelayTaskOwner {
    fn drop(&mut self) {
        let tasks = std::mem::take(
            self.tasks
                .get_mut()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
        );
        drop(tasks);
    }
}

pub(crate) struct DesktopRelayRuntimeExecutor {
    owner: DesktopRelayTaskOwner,
    next_generation: AtomicU64,
    event_sender: Sender<DesktopRelayRuntimeEvent>,
    event_receiver: Mutex<Receiver<DesktopRelayRuntimeEvent>>,
}

// The executor is managed now, while start/stop commands stay intentionally unpublished.
#[allow(dead_code)]
impl DesktopRelayRuntimeExecutor {
    fn new() -> Self {
        let (event_sender, event_receiver) = mpsc::channel();
        Self {
            owner: DesktopRelayTaskOwner::default(),
            next_generation: AtomicU64::new(1),
            event_sender,
            event_receiver: Mutex::new(event_receiver),
        }
    }

    pub(crate) fn execute(
        &self,
        account_id: &RelayAccountId,
        action: RelayLifecycleAction,
        connection_task: Option<DesktopRelayConnectionTask>,
    ) -> Result<Option<u64>, DesktopRelayRuntimeError> {
        match (action, connection_task) {
            (RelayLifecycleAction::StartConnection, Some(task)) => {
                self.start_connection(account_id, task).map(Some)
            }
            (RelayLifecycleAction::StartConnection, None) => {
                Err(DesktopRelayRuntimeError::ConnectionTaskRequired)
            }
            (RelayLifecycleAction::ScheduleRetry { delay_ms }, None) => {
                self.schedule_retry(account_id, delay_ms).map(Some)
            }
            (RelayLifecycleAction::CloseConnection | RelayLifecycleAction::CancelRetry, None) => {
                self.stop(account_id);
                Ok(None)
            }
            (RelayLifecycleAction::None, None) => Ok(None),
            (_, Some(_)) => Err(DesktopRelayRuntimeError::UnexpectedConnectionTask),
        }
    }

    pub(crate) fn try_next_event(&self) -> Option<DesktopRelayRuntimeEvent> {
        let receiver = self
            .event_receiver
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        loop {
            match receiver.try_recv() {
                Ok(event) => {
                    if let Some(event) = self.accept_current_event(event) {
                        return Some(event);
                    }
                }
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => return None,
            }
        }
    }

    pub(crate) fn stop(&self, account_id: &RelayAccountId) -> bool {
        self.owner.stop(account_id)
    }

    pub(crate) fn shutdown(&self) {
        self.owner.shutdown();
    }

    fn start_connection(
        &self,
        account_id: &RelayAccountId,
        task: DesktopRelayConnectionTask,
    ) -> Result<u64, DesktopRelayRuntimeError> {
        let generation = self.allocate_generation();
        let account_name = account_id.as_str().to_owned();
        let event_sender = self.event_sender.clone();
        self.owner.insert_with(account_id, generation, move || {
            DesktopRelayTaskHandle::spawn(generation, move |cancellation| {
                let mut context = DesktopRelayTaskContext {
                    account_id: account_name,
                    generation,
                    cancellation,
                    event_sender,
                    connected: false,
                };
                let error_code = task(&mut context);
                if !context.cancellation.is_cancelled() {
                    let kind = if context.connected {
                        DesktopRelayRuntimeEventKind::UnexpectedDisconnect(error_code)
                    } else {
                        DesktopRelayRuntimeEventKind::ConnectionFailed(error_code)
                    };
                    context.send(kind);
                }
            })
        })?;
        Ok(generation)
    }

    fn schedule_retry(
        &self,
        account_id: &RelayAccountId,
        delay_ms: u64,
    ) -> Result<u64, DesktopRelayRuntimeError> {
        let generation = self.allocate_generation();
        let account_name = account_id.as_str().to_owned();
        let event_sender = self.event_sender.clone();
        self.owner.insert_with(account_id, generation, move || {
            DesktopRelayTaskHandle::spawn(generation, move |cancellation| {
                if !cancellation.wait_timeout(Duration::from_millis(delay_ms)) {
                    let _ = event_sender.send(DesktopRelayRuntimeEvent {
                        account_id: account_name,
                        generation,
                        kind: DesktopRelayRuntimeEventKind::RetryElapsed,
                    });
                }
            })
        })?;
        Ok(generation)
    }

    fn accept_current_event(
        &self,
        event: DesktopRelayRuntimeEvent,
    ) -> Option<DesktopRelayRuntimeEvent> {
        if event.is_terminal() {
            if !self
                .owner
                .stop_if_generation(event.account_id(), event.generation())
            {
                return None;
            }
        } else if !self
            .owner
            .is_current(event.account_id(), event.generation())
        {
            return None;
        }
        Some(event)
    }

    fn allocate_generation(&self) -> u64 {
        self.next_generation.fetch_add(1, Ordering::Relaxed)
    }

    fn next_event_timeout(&self, timeout: Duration) -> Option<DesktopRelayRuntimeEvent> {
        let deadline = Instant::now() + timeout;
        let receiver = self
            .event_receiver
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            match receiver.recv_timeout(remaining) {
                Ok(event) => {
                    if let Some(event) = self.accept_current_event(event) {
                        return Some(event);
                    }
                }
                Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => return None,
            }
        }
    }

    #[cfg(test)]
    fn active_count(&self) -> usize {
        self.owner.active_count()
    }
}

struct OrchestratedAccount {
    lifecycle: RelayLifecycle,
    task_factory: DesktopRelayConnectionTaskFactory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopRelayOrchestratorError {
    Lifecycle(RelayLifecycleTransitionError),
    Runtime(DesktopRelayRuntimeError),
}

impl From<RelayLifecycleTransitionError> for DesktopRelayOrchestratorError {
    fn from(error: RelayLifecycleTransitionError) -> Self {
        Self::Lifecycle(error)
    }
}

impl From<DesktopRelayRuntimeError> for DesktopRelayOrchestratorError {
    fn from(error: DesktopRelayRuntimeError) -> Self {
        Self::Runtime(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopRelayOrchestratorUpdate {
    event: DesktopRelayRuntimeEvent,
    status: RelayRuntimeStatus,
}

impl DesktopRelayOrchestratorUpdate {
    pub fn event(&self) -> &DesktopRelayRuntimeEvent {
        &self.event
    }

    pub fn status(&self) -> RelayRuntimeStatus {
        self.status
    }
}

struct DesktopRelayOrchestratorInner {
    executor: DesktopRelayRuntimeExecutor,
    accounts: Mutex<HashMap<String, OrchestratedAccount>>,
}

impl DesktopRelayOrchestratorInner {
    fn start(
        &self,
        account_id: &RelayAccountId,
        task_factory: DesktopRelayConnectionTaskFactory,
    ) -> Result<RelayRuntimeStatus, DesktopRelayOrchestratorError> {
        let mut accounts = self.lock_accounts();
        let account = accounts
            .entry(account_id.as_str().to_owned())
            .or_insert_with(|| OrchestratedAccount {
                lifecycle: RelayLifecycle::new(),
                task_factory: Arc::clone(&task_factory),
            });
        let action = account.lifecycle.start()?;
        account.task_factory = task_factory;
        if let Err(error) = self.execute_action(account_id, account, action) {
            account.lifecycle.runtime_failed();
            return Err(error.into());
        }
        Ok(account.lifecycle.status())
    }

    fn apply_event(
        &self,
        event: DesktopRelayRuntimeEvent,
    ) -> Result<Option<DesktopRelayOrchestratorUpdate>, DesktopRelayOrchestratorError> {
        let account_id = RelayAccountId::new(event.account_id().to_owned())
            .expect("runtime events only contain validated Relay account IDs");
        let mut accounts = self.lock_accounts();
        let Some(account) = accounts.get_mut(account_id.as_str()) else {
            return Ok(None);
        };

        let action = match event.kind() {
            DesktopRelayRuntimeEventKind::Connected => {
                account.lifecycle.connected()?;
                RelayLifecycleAction::None
            }
            DesktopRelayRuntimeEventKind::ConnectionFailed(error_code) => {
                account.lifecycle.connection_failed(error_code)?
            }
            DesktopRelayRuntimeEventKind::UnexpectedDisconnect(error_code) => {
                account.lifecycle.unexpected_disconnect(error_code)?
            }
            DesktopRelayRuntimeEventKind::RetryElapsed => account.lifecycle.retry_elapsed()?,
        };

        if let Err(error) = self.execute_action(&account_id, account, action) {
            account.lifecycle.runtime_failed();
            return Err(error.into());
        }
        Ok(Some(DesktopRelayOrchestratorUpdate {
            event,
            status: account.lifecycle.status(),
        }))
    }

    fn status(&self, account_id: &RelayAccountId) -> RelayRuntimeStatus {
        self.lock_accounts().get(account_id.as_str()).map_or_else(
            || RelayLifecycle::new().status(),
            |account| account.lifecycle.status(),
        )
    }

    fn stop(&self, account_id: &RelayAccountId) -> Result<bool, DesktopRelayOrchestratorError> {
        let mut accounts = self.lock_accounts();
        let Some(account) = accounts.get_mut(account_id.as_str()) else {
            return Ok(false);
        };
        let action = account.lifecycle.stop();
        self.execute_action(account_id, account, action)?;
        if account.lifecycle.status().phase() == RelayRuntimePhase::Stopping {
            account.lifecycle.stopped()?;
        }
        accounts.remove(account_id.as_str());
        Ok(true)
    }

    fn shutdown(&self) {
        self.executor.shutdown();
        self.lock_accounts().clear();
    }

    fn execute_action(
        &self,
        account_id: &RelayAccountId,
        account: &OrchestratedAccount,
        action: RelayLifecycleAction,
    ) -> Result<(), DesktopRelayRuntimeError> {
        let task = if action == RelayLifecycleAction::StartConnection {
            Some((account.task_factory)())
        } else {
            None
        };
        self.executor.execute(account_id, action, task).map(drop)
    }

    fn lock_accounts(&self) -> MutexGuard<'_, HashMap<String, OrchestratedAccount>> {
        self.accounts
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

type OrchestratorUpdateResult =
    Result<DesktopRelayOrchestratorUpdate, DesktopRelayOrchestratorError>;

pub struct DesktopRelayOrchestrator {
    inner: Arc<DesktopRelayOrchestratorInner>,
    update_receiver: Mutex<Receiver<OrchestratorUpdateResult>>,
    pump_cancellation: DesktopRelayCancellation,
    pump: Mutex<Option<JoinHandle<()>>>,
    pump_available: AtomicBool,
}

pub fn desktop_relay_orchestrator() -> DesktopRelayOrchestrator {
    let inner = Arc::new(DesktopRelayOrchestratorInner {
        executor: DesktopRelayRuntimeExecutor::new(),
        accounts: Mutex::new(HashMap::new()),
    });
    let (update_sender, update_receiver) = mpsc::channel();
    let pump_cancellation = DesktopRelayCancellation::default();
    let pump_inner = Arc::clone(&inner);
    let pump_token = pump_cancellation.clone();
    let pump = thread::Builder::new()
        .name("moe-relay-event-pump".to_owned())
        .spawn(move || {
            while !pump_token.is_cancelled() {
                let Some(event) = pump_inner
                    .executor
                    .next_event_timeout(Duration::from_millis(100))
                else {
                    continue;
                };
                match pump_inner.apply_event(event) {
                    Ok(Some(update)) => {
                        if update_sender.send(Ok(update)).is_err() {
                            return;
                        }
                    }
                    Ok(None) => {}
                    Err(error) => {
                        if update_sender.send(Err(error)).is_err() {
                            return;
                        }
                    }
                }
            }
        })
        .ok();
    let pump_available = AtomicBool::new(pump.is_some());

    DesktopRelayOrchestrator {
        inner,
        update_receiver: Mutex::new(update_receiver),
        pump_cancellation,
        pump: Mutex::new(pump),
        pump_available,
    }
}

impl DesktopRelayOrchestrator {
    pub fn start(
        &self,
        account_id: &RelayAccountId,
        task_factory: DesktopRelayConnectionTaskFactory,
    ) -> Result<RelayRuntimeStatus, DesktopRelayOrchestratorError> {
        if !self.pump_available.load(Ordering::Acquire) {
            return Err(DesktopRelayRuntimeError::WorkerUnavailable.into());
        }
        self.inner.start(account_id, task_factory)
    }

    pub fn poll_next(
        &self,
    ) -> Result<Option<DesktopRelayOrchestratorUpdate>, DesktopRelayOrchestratorError> {
        match self
            .update_receiver
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .try_recv()
        {
            Ok(result) => result.map(Some),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => {
                Err(DesktopRelayRuntimeError::WorkerUnavailable.into())
            }
        }
    }

    pub fn status(&self, account_id: &RelayAccountId) -> RelayRuntimeStatus {
        self.inner.status(account_id)
    }

    pub fn stop(&self, account_id: &RelayAccountId) -> Result<bool, DesktopRelayOrchestratorError> {
        self.inner.stop(account_id)
    }

    pub fn shutdown(&self) {
        self.pump_available.store(false, Ordering::Release);
        self.pump_cancellation.cancel();
        if let Some(pump) = self
            .pump
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
        {
            let _ = pump.join();
        }
        self.inner.shutdown();
    }
}

impl Drop for DesktopRelayOrchestrator {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moe_relay_client::{RelayLifecycle, RelayRuntimePhase};
    use std::sync::atomic::AtomicUsize;

    const EVENT_TIMEOUT: Duration = Duration::from_secs(2);

    fn account_id(value: &str) -> RelayAccountId {
        RelayAccountId::new(value).unwrap()
    }

    #[test]
    fn maps_start_connected_stop_and_join_to_real_background_task() {
        let executor = DesktopRelayRuntimeExecutor::new();
        let account = account_id("primary-relay");
        let mut lifecycle = RelayLifecycle::new();
        let start = lifecycle.start().unwrap();

        let generation = executor
            .execute(
                &account,
                start,
                Some(Box::new(|context| {
                    context.report_connected();
                    context.cancellation().wait_cancelled();
                    RelayConnectionErrorCode::Cancelled
                })),
            )
            .unwrap()
            .unwrap();
        let connected = executor.next_event_timeout(EVENT_TIMEOUT).unwrap();
        assert_eq!(connected.generation(), generation);
        assert_eq!(connected.kind(), DesktopRelayRuntimeEventKind::Connected);
        lifecycle.connected().unwrap();
        assert_eq!(executor.active_count(), 1);

        let stop = lifecycle.stop();
        assert_eq!(stop, RelayLifecycleAction::CloseConnection);
        executor.execute(&account, stop, None).unwrap();
        lifecycle.stopped().unwrap();

        assert_eq!(lifecycle.status().phase(), RelayRuntimePhase::Offline);
        assert_eq!(executor.active_count(), 0);
        assert!(executor.try_next_event().is_none());
    }

    #[test]
    fn reports_initial_failure_and_unexpected_disconnect_separately() {
        let executor = DesktopRelayRuntimeExecutor::new();
        let initial = account_id("initial-relay");
        executor
            .execute(
                &initial,
                RelayLifecycleAction::StartConnection,
                Some(Box::new(|_| RelayConnectionErrorCode::RelayUnavailable)),
            )
            .unwrap();
        assert_eq!(
            executor.next_event_timeout(EVENT_TIMEOUT).unwrap().kind(),
            DesktopRelayRuntimeEventKind::ConnectionFailed(
                RelayConnectionErrorCode::RelayUnavailable
            )
        );

        let connected = account_id("connected-relay");
        executor
            .execute(
                &connected,
                RelayLifecycleAction::StartConnection,
                Some(Box::new(|context| {
                    context.report_connected();
                    RelayConnectionErrorCode::Protocol
                })),
            )
            .unwrap();
        assert_eq!(
            executor.next_event_timeout(EVENT_TIMEOUT).unwrap().kind(),
            DesktopRelayRuntimeEventKind::Connected
        );
        assert_eq!(
            executor.next_event_timeout(EVENT_TIMEOUT).unwrap().kind(),
            DesktopRelayRuntimeEventKind::UnexpectedDisconnect(RelayConnectionErrorCode::Protocol)
        );
    }

    #[test]
    fn maps_retry_action_to_real_timer_and_releases_finished_handle() {
        let executor = DesktopRelayRuntimeExecutor::new();
        let account = account_id("primary-relay");
        let generation = executor
            .execute(
                &account,
                RelayLifecycleAction::ScheduleRetry { delay_ms: 10 },
                None,
            )
            .unwrap()
            .unwrap();

        let event = executor.next_event_timeout(EVENT_TIMEOUT).unwrap();
        assert_eq!(event.account_id(), account.as_str());
        assert_eq!(event.generation(), generation);
        assert_eq!(event.kind(), DesktopRelayRuntimeEventKind::RetryElapsed);
        assert_eq!(executor.active_count(), 0);
    }

    #[test]
    fn cancel_retry_wakes_timer_without_waiting_for_deadline() {
        let executor = DesktopRelayRuntimeExecutor::new();
        let account = account_id("primary-relay");
        executor
            .execute(
                &account,
                RelayLifecycleAction::ScheduleRetry { delay_ms: 30_000 },
                None,
            )
            .unwrap();

        let started = Instant::now();
        executor
            .execute(&account, RelayLifecycleAction::CancelRetry, None)
            .unwrap();

        assert!(started.elapsed() < Duration::from_secs(1));
        assert_eq!(executor.active_count(), 0);
        assert!(
            executor
                .next_event_timeout(Duration::from_millis(25))
                .is_none()
        );
    }

    #[test]
    fn stale_timer_event_cannot_start_a_new_generation() {
        let executor = DesktopRelayRuntimeExecutor::new();
        let account = account_id("primary-relay");
        let first_generation = executor
            .execute(
                &account,
                RelayLifecycleAction::ScheduleRetry { delay_ms: 5 },
                None,
            )
            .unwrap()
            .unwrap();
        thread::sleep(Duration::from_millis(25));
        executor.stop(&account);

        let second_generation = executor
            .execute(
                &account,
                RelayLifecycleAction::ScheduleRetry { delay_ms: 5 },
                None,
            )
            .unwrap()
            .unwrap();
        let event = executor.next_event_timeout(EVENT_TIMEOUT).unwrap();

        assert_ne!(first_generation, second_generation);
        assert_eq!(event.generation(), second_generation);
        assert_eq!(event.kind(), DesktopRelayRuntimeEventKind::RetryElapsed);
        assert!(executor.try_next_event().is_none());
    }

    #[test]
    fn duplicate_start_does_not_spawn_the_second_task() {
        let executor = DesktopRelayRuntimeExecutor::new();
        let account = account_id("primary-relay");
        let second_started = Arc::new(AtomicBool::new(false));
        executor
            .execute(
                &account,
                RelayLifecycleAction::StartConnection,
                Some(Box::new(|context| {
                    context.report_connected();
                    context.cancellation().wait_cancelled();
                    RelayConnectionErrorCode::Cancelled
                })),
            )
            .unwrap();
        executor.next_event_timeout(EVENT_TIMEOUT).unwrap();

        let marker = Arc::clone(&second_started);
        assert_eq!(
            executor.execute(
                &account,
                RelayLifecycleAction::StartConnection,
                Some(Box::new(move |_| {
                    marker.store(true, Ordering::SeqCst);
                    RelayConnectionErrorCode::Cancelled
                })),
            ),
            Err(DesktopRelayRuntimeError::AlreadyActive)
        );
        assert!(!second_started.load(Ordering::SeqCst));
        executor.stop(&account);
    }

    #[test]
    fn shutdown_cancels_and_joins_every_account_task() {
        let executor = DesktopRelayRuntimeExecutor::new();
        let stopped = Arc::new(AtomicUsize::new(0));
        for account in [account_id("primary-relay"), account_id("backup-relay")] {
            let stopped = Arc::clone(&stopped);
            executor
                .execute(
                    &account,
                    RelayLifecycleAction::StartConnection,
                    Some(Box::new(move |context| {
                        context.report_connected();
                        context.cancellation().wait_cancelled();
                        stopped.fetch_add(1, Ordering::SeqCst);
                        RelayConnectionErrorCode::Cancelled
                    })),
                )
                .unwrap();
        }
        executor.next_event_timeout(EVENT_TIMEOUT).unwrap();
        executor.next_event_timeout(EVENT_TIMEOUT).unwrap();

        executor.shutdown();

        assert_eq!(executor.active_count(), 0);
        assert_eq!(stopped.load(Ordering::SeqCst), 2);
        assert!(executor.try_next_event().is_none());
    }

    #[test]
    fn dropping_executor_is_the_app_shutdown_fallback() {
        let stopped = Arc::new(AtomicBool::new(false));
        {
            let executor = DesktopRelayRuntimeExecutor::new();
            let stopped_by_task = Arc::clone(&stopped);
            executor
                .execute(
                    &account_id("primary-relay"),
                    RelayLifecycleAction::StartConnection,
                    Some(Box::new(move |context| {
                        context.report_connected();
                        context.cancellation().wait_cancelled();
                        stopped_by_task.store(true, Ordering::SeqCst);
                        RelayConnectionErrorCode::Cancelled
                    })),
                )
                .unwrap();
            executor.next_event_timeout(EVENT_TIMEOUT).unwrap();
        }

        assert!(stopped.load(Ordering::SeqCst));
    }

    #[test]
    fn cancel_hook_interrupts_task_before_join() {
        let executor = DesktopRelayRuntimeExecutor::new();
        let account = account_id("primary-relay");
        let hook_called = Arc::new(AtomicBool::new(false));
        let hook_marker = Arc::clone(&hook_called);
        executor
            .execute(
                &account,
                RelayLifecycleAction::StartConnection,
                Some(Box::new(move |context| {
                    context.cancellation().on_cancel(move || {
                        hook_marker.store(true, Ordering::SeqCst);
                    });
                    context.report_connected();
                    context.cancellation().wait_cancelled();
                    RelayConnectionErrorCode::Cancelled
                })),
            )
            .unwrap();
        executor.next_event_timeout(EVENT_TIMEOUT).unwrap();

        executor.stop(&account);

        assert!(hook_called.load(Ordering::SeqCst));
        assert_eq!(executor.active_count(), 0);
    }

    #[test]
    fn orchestrator_pump_retries_without_caller_polling() {
        let orchestrator = desktop_relay_orchestrator();
        let account = account_id("primary-relay");
        let attempts = Arc::new(AtomicUsize::new(0));
        let attempts_for_factory = Arc::clone(&attempts);
        let factory: DesktopRelayConnectionTaskFactory = Arc::new(move || {
            let attempt = attempts_for_factory.fetch_add(1, Ordering::SeqCst);
            Box::new(move |context| {
                if attempt == 0 {
                    RelayConnectionErrorCode::RelayUnavailable
                } else {
                    context.report_connected();
                    context.cancellation().wait_cancelled();
                    RelayConnectionErrorCode::Cancelled
                }
            })
        });

        let status = orchestrator.start(&account, factory).unwrap();
        assert_eq!(status.phase(), RelayRuntimePhase::Connecting);
        let deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < deadline
            && orchestrator.status(&account).phase() != RelayRuntimePhase::Connected
        {
            thread::sleep(Duration::from_millis(5));
        }

        let mut saw_retry_wait = false;
        while let Some(update) = orchestrator.poll_next().unwrap() {
            saw_retry_wait |= update.status().phase() == RelayRuntimePhase::RetryWaiting;
        }
        assert!(saw_retry_wait);
        assert_eq!(attempts.load(Ordering::SeqCst), 2);
        assert_eq!(
            orchestrator.status(&account).phase(),
            RelayRuntimePhase::Connected
        );
        assert!(orchestrator.stop(&account).unwrap());
        assert_eq!(
            orchestrator.status(&account).phase(),
            RelayRuntimePhase::Offline
        );
    }

    #[test]
    fn action_rejects_missing_or_unexpected_connection_task() {
        let executor = DesktopRelayRuntimeExecutor::new();
        let account = account_id("primary-relay");
        assert_eq!(
            executor.execute(&account, RelayLifecycleAction::StartConnection, None),
            Err(DesktopRelayRuntimeError::ConnectionTaskRequired)
        );
        assert_eq!(
            executor.execute(
                &account,
                RelayLifecycleAction::None,
                Some(Box::new(|_| RelayConnectionErrorCode::Cancelled)),
            ),
            Err(DesktopRelayRuntimeError::UnexpectedConnectionTask)
        );
    }
}

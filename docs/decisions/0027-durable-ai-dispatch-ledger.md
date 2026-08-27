# ADR 0027: Durable per-recipient AI dispatch ledger

- Status: Accepted
- Date: 2026-08-13
- Depends on: ADR 0012 (idempotent Room message writes), ADR 0025 (single desktop instance), ADR 0026 (per-recipient dispatch results)

## Context

M.O.E. saves a human message before asking an external AI for a reply. The
first dispatch guard lived only in process memory. If the desktop process
stopped after an adapter had started but before the reply was saved, a restart
could no longer distinguish "never delivered" from "possibly delivered".
Blindly retrying that source message could therefore spend a second provider
turn and create a duplicate reply.

Codex and Grok now share the same multi-recipient command, so this boundary
must be recorded per source message and recipient. A successful recipient must
remain visible even when another recipient has an ambiguous result.

## Decision

1. Persist a device-local JSON dispatch ledger in the Tauri app-data directory.
   It is operational state and is not included in portable Room backups.
2. Use the stable key `(sourceMessageId, recipientId)` and record the Room ID,
   deterministic reply message ID, state, and update time. The states are:
   `prepared`, `externalStarted`, `completed`, and `failed`.
3. Persist `prepared` before local request preparation and persist
   `externalStarted` immediately before invoking an external adapter. If that
   second write fails, the adapter is not invoked.
4. Any failure after `externalStarted` is an `unknown` outcome. M.O.E. never
   automatically resends it. The UI says that the message may have reached the
   provider and that automatic resend was suppressed.
   The active Room also reads and shows unresolved records after app startup or
   when the user switches Rooms.
5. A deterministic reply already present in the Room is authoritative recovery
   evidence, even if the ledger still says `externalStarted`. It is returned as
   a duplicate without another provider call.
6. A `prepared` record found after restart is a definite pre-delivery failure;
   it is not automatically resumed. A `completed` record whose Room reply is
   missing is also not resent because the external turn already completed.
7. Keep unresolved records. Bound the file and entry count, and prune only the
   oldest terminal (`completed` or `failed`) records when capacity is needed.
8. Keep one application writer. SQLite, cross-process locking, provider-side
   lookup, and automatic reconciliation are outside this tranche.

## Consequences

- A crash or timeout cannot silently turn into an automatic duplicate external
  turn after restart.
- Users may occasionally see a conservative "possibly delivered" warning even
  if the process stopped in the small interval between recording
  `externalStarted` and invoking the adapter.
- The Room transcript remains the source of truth for saved replies, while the
  ledger records only delivery safety state.
- Manual reconciliation and a user-visible command to end tracking remain
  future work; this tranche provides the durable evidence and safe UI wording.

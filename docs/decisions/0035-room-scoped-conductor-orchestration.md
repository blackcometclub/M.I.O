# ADR 0035: Room-scoped conductor orchestration

- Status: Accepted
- Date: 2026-08-14
- Depends on: ADR 0012 (idempotent Room message writes), ADR 0020
  (Room-owned AI continuity), ADR 0023 (participant identity), ADR 0027
  (durable per-recipient dispatch ledger), ADR 0030 (parallel recipient
  dispatch), ADR 0034 (device-local AI access permissions)

## Context

M.I.O. can currently save one Owner message and dispatch it directly to one or
more selected AI participants. Each selected participant receives an
independent provider turn. Replies from the same dispatch wave are not shared
with the other recipients until a later Room turn.

The Owner also wants an orchestration mode in which one selected AI acts as a
conductor: it decides which Room participants should handle bounded subtasks,
receives their results, and returns one integrated answer. The conductor must
not impersonate the Owner, gain Owner settings authority, or silently inherit
general MCP, network, filesystem, or Computer Use access.

This is different from both of the following features:

- **General tool access:** an AI uses MCP, web, filesystem, or command tools.
- **Owner proxy operation:** Codex temporarily operates the Owner UI through
  Computer Use.

Neither is required for Room orchestration and neither is accepted by this ADR.

## Proposed decision

### Keep identity, authority, and transport separate

The Room stores the immutable participant ID of the selected conductor. The
conductor remains an AI participant and all of its messages use that participant
ID. It cannot author an Owner message or change Room settings.

M.I.O. Core, not the conductor model, owns every side effect. The conductor may
only propose a validated plan. Core decides whether the plan is valid, creates
deterministic delegation messages, invokes existing participant adapters, saves
their replies, and requests the final synthesis.

The provider-neutral capability is called `conductorPlanV1`. An adapter that
does not advertise and pass product-path tests for this capability cannot be
selected as a conductor. The first product implementation may enable Codex
only; Gemini, Claude Fable, Grok, and future adapters remain visible but
disabled until separately verified.

### Make orchestration an explicit send mode

A Room may have zero or one configured conductor participant. Configuration is
device-local and keyed by Room ID because available adapters and verified
capabilities are device-specific. It is excluded from portable Room backups in
the first version. Restoring a Room on another device therefore defaults to no
conductor.

The composer has two explicit modes when a conductor is configured:

- **Direct:** retain the current selected-recipient behavior.
- **Conductor:** address the saved Owner message only to the configured
  conductor and start one orchestration operation.

The two modes remain selectable for every send; configuring a conductor never
locks the Room into Conductor mode. The UI must show the active mode before
send and store the last selected mode device-locally per Room. Immediately
after a conductor is first configured, a composer with no existing draft starts
in Conductor mode. Selecting a conductor must not silently reinterpret an
existing draft that was composed in Direct mode. Removing the conductor from
the Room or losing its verified capability clears the effective selection and
falls back to Direct mode.

The configured conductor has a small dedicated conductor badge beside its
participant avatar wherever Room participants or selected recipients are
shown. The badge uses the product's conductor symbol, not provider branding,
and is visually distinct from readiness, installation, and selected-recipient
indicators. It exposes the label `Conductor` through a tooltip and accessible
name, so the role is not communicated by the icon alone.

### Use a bounded two-phase conductor protocol

The first protocol has one planning turn and, when delegation is requested, one
synthesis turn.

1. Save the Owner message before any external AI call.
2. Ask the conductor for a strict `ConductorPlanV1` envelope.
3. Validate the complete envelope before invoking any worker adapter.
4. Create and save one deterministic conductor-to-worker message for each
   accepted delegation.
5. Dispatch those worker messages through the existing per-recipient adapter,
   continuity, timeout, and durable-ledger path. Different providers may run in
   parallel under ADR 0030.
6. Save each worker reply as a normal worker-to-conductor Room message.
7. Give the conductor an explicit result packet containing every worker status
   and saved reply that is available.
8. Save one conductor-to-Owner final answer before marking the operation
   complete.

The planning envelope is data, not executable instructions:

```json
{
  "version": 1,
  "mode": "answer | delegate",
  "directAnswer": "string or null",
  "delegations": [
    {
      "targetParticipantId": "gemini",
      "task": "bounded task text"
    }
  ]
}
```

Core rejects unknown fields, invalid UTF-8, excessive byte lengths, duplicate
targets, the Owner, the conductor itself, non-members, human participants, and
unsupported targets. `answer` requires one bounded direct answer and no
delegations. `delegate` requires one to three unique worker delegations and no
direct answer.

The synthesis output is one bounded plain-text answer. Worker results and Room
history are untrusted context and do not change system, developer, permission,
or response-language rules.

### Bound the operation and prohibit autonomous loops

Version 1 has the following fixed limits:

- one conductor;
- one delegation round;
- at most three unique workers;
- at most one worker turn per selected worker;
- one final synthesis turn;
- no worker-to-worker or nested conductor delegation;
- no automatic background start;
- no automatic retry after an adapter may have started externally.

An orchestration operation starts only from a saved Owner message sent in
Conductor mode. The model cannot schedule a later operation or keep running
after completion.

### Preserve delivery safety and provenance

Use a stable operation ID derived from the saved source message ID and conductor
ID. Derive each delegation message ID, worker dispatch ID, and final message ID
from the operation ID, round, target participant ID, and ordinal. Retrying a
local command must therefore recover or return the same records rather than
create another provider turn.

The existing per-recipient dispatch ledger remains authoritative for each
worker invocation. An `externalStarted` worker with no saved reply is reported
to the conductor as `unknown`; it is never retried automatically. A failed or
unsupported worker is also represented explicitly in the synthesis packet so
the conductor can finish with partial evidence.

Add a separate device-local orchestration ledger for operation stage and the
message IDs it created. Proposed stages are `prepared`, `planning`,
`delegating`, `synthesizing`, `completed`, `failed`, and `unknown`. The Room
transcript remains the source of truth for saved messages. The orchestration
ledger is operational state and is not exported in Room backups.

Delegation messages and worker replies remain ordinary Room messages, so they
are still understandable if the device-local orchestration ledger is absent.
The UI may group those known message IDs into a collapsible operation trace,
but it must not hide them from durable history or imply that a missing reply was
received.

### Isolate conductor continuity from ordinary participant continuity

Planning and synthesis belong to one operation-scoped conductor session. They
must not reuse the conductor's ordinary direct-chat native session. If native
resume is available, synthesis may resume the operation session. If it cannot
resume, M.I.O. starts a fresh synthesis turn with a bounded explicit packet
containing the Owner request, accepted plan, and worker results.

Only the final conductor answer is a normal conductor-to-Owner reply. A future
direct message to that participant learns about the operation from the durable
Room transcript, not from an ambiguous partially advanced native session.

## First implementation boundary after acceptance

The first code tranche is a provider-free Core contract test, not a live UI or
external-AI feature. It includes only:

1. `ConductorPlanV1` parsing and validation in Rust;
2. deterministic operation, delegation, and final-message ID derivation;
3. a pure orchestration state machine with fake conductor and worker adapters;
4. tests for direct answer, one-to-three workers, partial failure, unknown
   outcome, invalid plans, duplicate targets, self/Owner delegation, bounded
   execution, and idempotent re-entry;
5. a provider-neutral capability flag or contract seam without enabling any
   production adapter.

It does not change the Room snapshot schema, Tauri commands, renderer, Codex
prompts, continuity files, dispatch ledger, or any live provider invocation.
Those changes require a second explicitly approved product-path tranche after
the Core contract is reviewed.

## Deferred

- General MCP, web, network, filesystem, command, or Computer Use access.
- Owner proxy mode or messages authored as the Owner.
- Multiple conductor rounds, plan revision, voting, debate, or nested teams.
- More than three workers or provider-defined unbounded fan-out.
- Choosing Gemini, Claude Fable, Grok, or another adapter as conductor.
- Automatic conductor selection or background orchestration.
- Portable conductor settings and cross-device orchestration-ledger recovery.
- Hidden chain-of-thought. The product stores only explicit delegation tasks,
  statuses, replies, and the final answer.

## Consequences

- The Owner can ask one AI to coordinate the Room without giving it Owner
  identity or arbitrary machine access.
- Existing direct multi-recipient sending remains available and unchanged.
- Core validates every recipient and side effect, so malformed model output
  cannot silently expand authority.
- The operation consumes an additional conductor planning turn and usually a
  conductor synthesis turn, increasing latency and provider usage.
- Partial and unknown worker outcomes remain visible instead of being retried or
  rewritten as successful collaboration.
- Device-local conductor selection is conservative but must be reconfigured
  after moving a Room to another device.

## Accepted first-release choices

- One round with at most three workers is sufficient for the first release.
- The composer remains switchable between Direct and Conductor mode, remembers
  the last mode per Room, and starts in Conductor mode only when a conductor is
  first configured with no existing draft.
- The first product-path UI groups delegation messages under a collapsible
  operation trace by default. The durable messages remain inspectable.
- The configured conductor receives a dedicated accessible badge beside its
  avatar.

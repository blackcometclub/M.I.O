# ADR 0029: Visible AI continuity degradation and local reset

- Status: Accepted
- Date: 2026-08-13
- Depends on: ADR 0020 (Room-owned AI continuity)

## Context

M.O.E. deliberately saves an AI reply to the Room before saving the provider's opaque continuation id. That protects conversation data, but a failure at the second step was previously invisible. The next turn could therefore start a fresh provider session without the user knowing that it had reconstructed context from the bounded Room transcript.

The recovery packet is also intentionally bounded to 16 messages and 800 Unicode characters per message. Those limits are safe, but silently applying them can make an AI appear to remember more than it actually received.

## Decision

### Every completed native turn reports its context state

Codex and Grok dispatch results include a provider-neutral report with:

- `initial`, `resumed`, or `reconstructed` mode;
- included and omitted message counts;
- the number of shortened long messages and omitted characters;
- whether the new continuation state was saved.

The same counts are included in the untrusted Room context packet sent to the AI. They do not become Room messages and are not exported in backups.

### Normal continuity stays quiet

The composer shows no extra status for an ordinary resumed turn. It shows a non-error notice only when M.O.E. reconstructed context, omitted history, shortened long bodies, or could not save the next continuation state.

A failed continuation save does not change the already-saved reply into a failed dispatch. The notice says that the reply is saved and that the next turn will restart from Room history.

### Reset clears only the local native binding

Room settings offers a reset for native Codex and Grok continuity. It removes only the selected `(roomId, participantId)` binding on this device. It does not delete Room messages, participant profiles, workspaces, or provider-side historical sessions. The next addressed turn starts a new native session and receives the bounded Room snapshot.

The reset is persisted atomically. If persistence fails, the in-memory binding remains unchanged and the UI reports that Room history was not modified.

## Consequences

- Users can distinguish durable native continuation from bounded Room reconstruction.
- Context loss and truncation are visible without making normal chat noisy.
- Display names may change, but reports remain keyed by stable participant ids.
- Reset is recoverable from the Room transcript, subject to the stated replay bounds.
- Summarization beyond the bounded replay window remains future work.

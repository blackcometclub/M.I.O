# ADR 0030: Parallel recipient dispatch and incremental replies

- Status: Accepted
- Date: 2026-08-13
- Depends on: ADR 0020 (Room-owned AI continuity), ADR 0027 (Durable AI dispatch ledger)

## Context

A Room message can address more than one AI participant. The desktop previously dispatched those recipients in sequence behind one shared native turn lock, then returned every result as one batch. A quick reply was therefore hidden until the slowest selected AI had also finished, and the total wait approximately accumulated across providers.

M.O.E. must improve that feedback without weakening the saved-message boundary, duplicate-delivery protection, provider continuity, or explicit unknown-outcome handling.

## Decision

### The saved Room message remains the dispatch authority

The human message and its recipient ids are saved before any external AI call. A single-recipient dispatch command may address only an AI already listed on that saved message. It cannot add or substitute a recipient from the renderer.

The existing batch command remains available for compatibility and delegates to the same single-recipient result path.

### Different native providers may run concurrently

Codex and Grok use separate provider turn gates. Different native participants may therefore process the same saved Room message concurrently, while repeated turns for the same provider remain serialized so that its continuation state cannot race with itself.

Each recipient retains its own durable ledger entry and continuity binding. Unsupported, failed, and unknown outcomes remain isolated to that recipient and are never converted into a reply from another AI.

### Replies appear independently

The renderer starts one dispatch request per saved recipient and inserts each successful reply as soon as that request completes. It does not wait to collect a display batch. The composer remains in the waiting state until every selected recipient has reached a result, then returns focus to the input field.

Replies started from the same human message are independent provider turns. They are not guaranteed to see another provider's reply from that same dispatch wave. Both saved replies are available in Room history on later turns.

## Consequences

- The first available AI response becomes visible without waiting for slower recipients.
- Total multi-AI latency approaches the slowest selected provider instead of the sum of provider latencies.
- Same-provider continuity remains ordered and durable.
- The UI can show one current typing participant while other recipient requests continue in parallel.
- Exact cross-provider simultaneous snapshots and a long-lived Codex process are separate future improvements.

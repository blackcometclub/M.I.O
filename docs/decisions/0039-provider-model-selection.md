# ADR 0039: Provider model selection

- Status: Accepted
- Date: 2026-08-31
- Depends on: ADR 0020 (Room-owned AI continuity), ADR 0023 (Participant identity and local profile overrides)

## Context

M.I.O. previously used each Provider's implicit default except for adapters that
hard-coded one tested model. The device owner wants to choose a model for each AI
without forcing a fixed model, inventing unsupported identifiers, or silently
switching to a different model when the selected one is unavailable.

## Decision

1. Store one device-local `aiModel` value in each participant profile. Missing
   values from existing version 1 profile files resolve to `providerDefault`.
2. `providerDefault` is the initial value and omits the Provider model argument.
   M.I.O. does not claim which model the Provider will choose in that mode.
3. Explicit values are not free-form. The Windows V1 allow list contains Codex
   `gpt-5.6-sol`, `gpt-5.6-terra`, and `gpt-5.6-luna`, Claude Code
   `claude-fable-5`, `claude-opus-5`, and `claude-sonnet-5`, and Grok `grok-4.6`.
   Gemini Antigravity remains Provider-default-only until a model flag and live
   response are validated. A model belonging to another Provider is invalid.
4. Pass the selected value through the provider-neutral text-turn request. Codex
   App Server receives `model` and disables authoritative Provider fallback.
   Claude Code and Grok receive `--model` only for an explicit selection. Every
   adapter rejects an unapproved value before starting its Provider process.
5. Include the explicit model in the Room continuity environment key. Changing
   the model starts a new Provider session on the next turn while preserving the
   M.I.O. Room history. The old session is not resumed under a new model.
6. Show the model beside the participant's locked Provider identity. Explain that
   availability, billing, and retention follow the Provider contract, and that
   M.I.O. does not silently choose a replacement model.
7. Add or remove an explicit candidate only after its current adapter argument,
   default behavior, response parsing, and continuity behavior have been tested.

## Consequences

- Existing users remain on Provider defaults unless they opt into a tested model.
- A retired, unavailable, or plan-ineligible explicit model fails visibly instead
  of changing response behavior or price without notice.
- Candidate lists require maintenance as Provider products change.
- Conductor and live-account regression tests remain required before V1 release.

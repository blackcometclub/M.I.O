# ADR 0031: Device-local AI participant guidance

- Status: Accepted
- Date: 2026-08-13
- Depends on: ADR 0023 (Participant identity and local profile overrides), ADR 0020 (Room-owned AI continuity)

## Context

Participant profiles already let the device owner change a display name and
avatar without changing the stable provider identity. Talk Room users also
need a simple way to give each AI a persistent tone, role, and form of address,
such as asking Gemini to answer energetically, without repeating that request
in every message.

## Decision

1. Add one optional `aiInstructions` text field to each device-local AI
   participant profile. Human profiles always save an empty value.
2. Bound the field to 2,000 Unicode characters and reject unsafe control
   characters. Existing version 1 profile files load with an empty default, so
   names and avatars remain compatible.
3. Present the field in the existing participant profile editor. It is local
   to this device and is not part of Room message history or participant
   identity.
4. Insert only the addressed AI's saved guidance into its provider prompt as
   trusted local configuration. Room messages remain separately labelled as
   untrusted conversational content.
5. Include a hash of the guidance in the AI continuity environment key. When
   guidance changes, the next turn reconstructs bounded Room history in a new
   provider conversation instead of resuming a session that retained the old
   guidance.
6. Guidance may control tone, role, and forms of address, but cannot override
   provider safety, the current user request, workspace boundaries, or tool and
   network restrictions.

## Consequences

- Codex, Grok, and Gemini can keep distinct locally configured personalities.
- Changing a personality takes effect on the next message without deleting
  Room history.
- Guidance is not synchronized to another device and is not a provider-wide
  account setting.

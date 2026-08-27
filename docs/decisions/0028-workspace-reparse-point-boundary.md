# ADR 0028: Workspace reparse-point boundary

- Status: Accepted
- Date: 2026-08-13
- Depends on: ADR 0019 (Room-scoped Codex workspace mode)

## Context

M.O.E. canonicalized the folder selected for Codex workspace mode, but did not
inspect whether that folder was a Windows junction, symbolic link, or another
reparse point. A saved ordinary folder could also be deleted and replaced by a
link before the next turn. Passing that path to a provider would make the
workspace boundary harder to explain and audit.

Recursively scanning every descendant before each turn is not practical. A
measurement of the M.O.E. development tree did not finish within 30 seconds.
It would also be racy because a descendant could change after the scan.

The Codex permission profile is intended to enforce the recursive workspace
boundary. An isolated live regression test for a nested junction was added,
but the current test environment returned `Unavailable` for both the ordinary
workspace control and junction case. Therefore nested-junction enforcement is
not recorded here as experimentally verified.

On 2026-08-26 the ordinary read/write control passed with Codex's native Windows
`elevated` sandbox. The nested-junction regression then denied a write through
the junction but returned the marker read from outside the selected root. This
confirmed that requiring `elevated` and rejecting a reparse-point root did not
by themselves satisfy the read boundary. OpenAI's permission-profile guidance
provides a workspace-only example that explicitly denies `:root`, then reopens
`:minimal` and the selected workspace roots.

The approved candidate added that explicit root deny. A second live attempt did
not produce a usable model response, so it was not treated as boundary evidence.
A model-free App Server `command/exec` diagnostic then applied the same custom
profile to an isolated junction fixture. It read the outside marker through the
junction, while the outside write remained denied. The explicit root deny is
therefore defense in depth, not a sufficient Windows nested-junction boundary.

## Decision

1. Reject a selected workspace root when its own filesystem metadata marks it
   as a Windows reparse point or a symbolic link on another platform.
2. Repeat the same root check immediately before each AI turn. If a previously
   saved ordinary folder is replaced by a link, fail closed before invoking the
   adapter.
3. Return a distinct `roomWorkspaceUnsafeLink` code and explain the rejection
   in the Room UI without exposing the local path.
4. Every M.I.O. Codex permission profile explicitly denies `:root`, then grants
   `:minimal` read access and grants each selected workspace root only the
   access required by that Room mode. Network remains disabled.
5. Disable Codex workspace read and write in the Windows alpha. The UI presents
   both modes as unavailable, legacy provider-default access resolves to
   chat-only, and an explicit saved workspace request fails before the provider
   process starts. Keep the selected-folder data model for future revalidation.
6. Keep an ignored, opt-in live Codex regression that creates only isolated
   temporary data and fails if a nested junction can read or write outside the
   selected root.
7. Do not claim nested-junction verification until the ordinary control and
   junction regression both run successfully in the same product environment.
   Do not replace this with an unbounded recursive scan.

## Consequences

- Selecting a junction or symlink directly is rejected, and replacing a saved
  root with one is detected before external delivery.
- Ordinary canonical directories keep their existing behavior and do not pay
  a full-tree scan cost before every message.
- The explicit root deny follows the documented least-privilege workspace-only
  profile shape, but does not make the Windows nested-junction claim sufficient.
- Windows alpha users keep Codex chat-only operation. Workspace controls remain
  visible but disabled, and stale explicit settings fail closed before delivery.
- Nested reparse points remain an explicit release-verification item governed
  by the Codex permission boundary, not a silently assumed app guarantee.

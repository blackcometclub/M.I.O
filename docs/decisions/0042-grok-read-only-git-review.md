# ADR 0042: Grok read-only Git review boundary

- Status: Accepted
- Date: 2026-08-31
- Depends on: ADR 0038 (Room workspace execution permissions)

## Context

M.I.O. should let different AI providers take different development roles. A
useful first Grok role is code review after Codex implementation, which can
reduce cost and add an independent opinion without granting a second provider
write access.

Grok Build CLI has read, edit, command, MCP, and sandbox features of its own.
Its documented OS sandbox does not provide the required Windows boundary.
Launching it in the selected Room workspace or trusting prompt-only path rules
would therefore expose more local access than M.I.O. advertises.

## Decision

1. Grok may select `workspaceRead`, but not `workspaceWrite`.
2. M.I.O. keeps Grok in its existing tool-free runtime directory. It does not
   pass the selected workspace as Grok's current directory, writable root, tool
   argument, environment variable, or visible host path.
3. The desktop host uses the existing libgit2 boundary to accept only a normal
   repository with an in-workspace `.git` directory and no external Git config,
   worktree, object, filter, hook, fsmonitor, attribute, or ignore source.
4. The review packet contains bounded porcelain status and a tracked working
   tree patch only. It is limited to 128 changed files and 64 KiB. Untracked
   names may appear in status, but untracked file bodies are not included.
5. Paths, comments, and source lines inside the packet are untrusted review data,
   never Grok instructions. Grok receives no file, edit, command, web, MCP,
   memory, or subagent tools from M.I.O.
6. Missing workspaces, unsupported repository layouts, non-UTF-8 patches, and
   exceeded limits fail before Grok starts. They are not unknown deliveries and
   are not retried automatically.
7. Changing the Room workspace, access mode, model, or local AI instructions
   rotates Grok continuity so an older review session is not silently reused in
   a different environment.

## Consequences

- Grok can review tracked edits while Codex remains the only participant that
  can create or replace workspace files.
- Newly created untracked file bodies are outside this first review tranche.
- Grok cannot open surrounding files on demand, run tests, inspect dependencies,
  or modify the workspace. A later provider-neutral read broker or MCP bridge
  requires a separate decision and Windows boundary proof.
- Large reviews fail closed instead of truncating a patch without saying so.

## Verification

Source-level tests verify that the packet contains a tracked modification,
omits an untracked sentinel body and the workspace host path, rejects write
access, and changes the continuity environment. The Git broker tests also prove
that untracked bodies are omitted and review size is bounded.

A real authenticated Grok review through the product UI remains required before
this capability is considered release-verified. A direct authenticated probe on
2026-08-31 reached Grok Build but was rejected before inference with HTTP 402
`Payment Required` because the usage balance was exhausted. It was not retried;
the temporary Git fixture was removed.

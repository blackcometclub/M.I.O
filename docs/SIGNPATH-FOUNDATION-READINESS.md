# SignPath Foundation readiness review

- Reviewed: 2026-09-03
- Scope: free SignPath Foundation code signing for the public M.I.O. Windows release
- Result: **Do not submit under the current licensing policy**
- External changes: none; no application or signing request was submitted

## Blocking conflict

The SignPath Foundation conditions for free OSS subscriptions require an
OSI-approved Open Source license **without commercial dual licensing** for all
components:

<https://signpath.org/terms.html>

M.I.O. is published under `AGPL-3.0-only`, but the copyright holder also offers a
separate paid commercial license for uses that need different terms. This is an
intentional product decision recorded in both:

- `COMMERCIAL-LICENSE.md`
- `docs/decisions/0004-agpl-commercial-dual-license.md`

Both files are tracked and included in the public source snapshot. The project
must therefore not claim eligibility for the free SignPath Foundation program or
submit an application that omits this licensing policy.

Removing the commercial-license option solely to obtain free signing would be a
material business and licensing change. It requires an explicit Owner decision
and is not part of release engineering.

## Other conditions reviewed

The following items appear compatible or can be prepared if the licensing
conflict is resolved:

- M.I.O. uses the OSI-approved AGPL-3.0-only license for its public code.
- The project is actively maintained and its functionality is documented.
- Public alpha source and release material already exist.
- The Windows installer provides an uninstall path and describes system changes.
- The release process builds from a fixed source commit and records hashes.

The following items would still need completion before applying:

- publish a dedicated **Code signing policy** page and link it from the project
  home page and download/release pages;
- name the project authors/committers, reviewers, and signing approvers;
- publish a privacy statement covering Provider-directed network transfers;
- confirm multi-factor authentication for repository and SignPath accounts;
- add a verifiable CI build and SignPath submission workflow after SignPath
  supplies the organization, project, policy, and artifact configuration values.

No placeholder workflow is added now. It could be run accidentally, it cannot be
validated without an accepted SignPath project, and GitHub Actions usage is being
reserved for the final release candidate.

## Viable alternatives without changing the license

1. Keep the existing NSIS/direct-download route and use a CA-trusted commercial
   code-signing certificate. ADR 0045 and the release scripts support the local
   Windows certificate-store route without recording private keys or passwords.
2. Investigate a separate Microsoft Store MSIX distribution. The Store signs
   accepted MSIX/AppX submissions, but it does not sign the current NSIS EXE.
   Moving M.I.O. to MSIX may change installation, update, Provider CLI, workspace,
   and sidecar behavior, so it requires an isolated feasibility test before it
   can replace the current V1 installer.
3. Keep publishing only release candidates while the signing route remains open.
   An unsigned installer may be published only when it is clearly labeled as an
   RC or Preview and accompanied by its SHA-256, source commit, and an explicit
   warning that its publisher cannot be verified. It must not be described as
   the final V1 artifact.

## Owner release sequence

On 2026-09-03 the Owner chose to keep commercial dual licensing while trying the
Microsoft Store MSIX route. A Store rejection must be reviewed first and fixed or
resubmitted when practical. Only if both Store signing and owner-funded signing
are impractical will the Owner separately decide between an unsigned stable V1
and ending commercial dual licensing before a SignPath application. Neither
fallback is authorized by this review alone.

## Resume condition

Resume SignPath preparation only if either:

- the Owner explicitly changes ADR 0004 and the commercial licensing policy; or
- SignPath Foundation gives written confirmation that M.I.O.'s current licensing
  arrangement is eligible.

Until then, continue V1 functional and packaging work without submitting a
SignPath application.

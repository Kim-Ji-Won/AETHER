# Git workflow and repository administration

## Trunk-based development

Every normal task starts from an Issue. Use short-lived topic branches from the reviewed main baseline; no long-running develop branch or unrelated feature bundling. Main is intended to represent reviewed, integration-tested work; enforcement is not yet active. The sole initial README commit was an authorized bootstrap exception (Issue #1 records its SHA). No later governance or application work may commit directly to main.

Branch formats: `feat/<issue>-<description>`, `fix/<issue>-<description>`, `spike/<issue>-<description>`, `chore/<issue>-<description>`, `docs/<issue>-<description>`, `test/<issue>-<description>`, `security/<issue>-<description>`, or `release/<version>` (still link its real Issue in the PR). Use actual Issue numbers and lowercase kebab-case descriptions. Never force-push shared branches, rewrite history or delete unmerged work.

Conventional Commit and PR title format: `type(scope): imperative summary`. Types: feat, fix, docs, test, refactor, perf, build, ci, chore, security. Use meaningful commit boundaries. Stage explicit file paths, inspect the staged diff, and keep lockfiles but exclude secrets and generated output.

Open a Draft PR for incomplete work. Use `Refs #N` for partial work and `Closes #N` only if the merged PR fully completes the Issue. Review the latest head SHA. Merge only after Issue criteria, required CI and local checks (or explicitly accepted exceptions), security review, resolved critical findings and explicit user merge approval. Review comments are not formal approval; green CI is not merge permission. Squash commit titles must follow Conventional Commits. After an authorized merge verify main SHA and local/remote synchronization.

## Observed settings and approval-dependent configuration

P0-G inspection on 2026-11-06: public repository, main initialized, no rulesets, main not protected, squash/merge/rebase enabled, auto-merge disabled, automatic branch deletion disabled. Admin capability was reported, but no settings were changed. Current state: **POLICY_DOCUMENTED_NOT_TECHNICALLY_ENFORCED**. Recheck live state before activation.

Proposed settings (explicit authorization required): squash ON; merge commits OFF; rebase OFF; auto-merge OFF; automatic head deletion ON only after confirming it will not remove needed work. No branch deletion is authorized by this document.

Proposed active main ruleset: target refs/heads/main; require PR; block force pushes and deletion; require conversation resolution; restrict direct updates through PR requirement; minimize and document bypass actors. Initially require zero independent approvals until a viable reviewer plan exists. Do not require signed commits without signing readiness. After the actual `governance` check succeeds reliably, select its exact check identity as required; do not require planned application checks. Test a maintainer's legitimate merge path before enforcing; do not configure an unconditional update restriction that blocks all merges. Record enforcement mode, bypass scope and evidence. Documentation alone does not protect main.

## Exact proposed label taxonomy

Create only missing labels after comparing existing names; never assign absent labels. Existing default labels are retained. This bootstrap does not administer labels; use the following manual configuration list. Colors omit the hash.

| Labels | Color | Description pattern |
|---|---|---|
| type:feature, type:bug, type:research, type:security, type:chore | 1d76db | Work type: corresponding suffix |
| area:desktop, area:ui, area:agent, area:voice, area:memory, area:native, area:security, area:release, area:infra | 5319e7 | Component: corresponding suffix |
| priority:p0 | b60205 | Blocking or critical priority |
| priority:p1 | d93f0b | Important next-priority work |
| priority:p2 | fbca04 | Normal or deferred priority |
| status:blocked | b60205 | Cannot proceed without a named dependency |
| status:needs-review | 0e8a16 | Awaiting review; not an approval |

## Proposed milestone titles

Create only missing titles, without due dates or invented numeric IDs. These group the roadmap, not delivery promises. Creation/assignment is pending maintainer configuration.

- P0 — Architecture & Feasibility
- P1 — Desktop Foundation
- P2 — Agent Runtime
- P3 — Voice Intelligence
- P4 — Computer Control
- P5 — Persistent Memory
- P6 — Product UI & Experience
- P7 — Release Engineering

Enable and verify Private Vulnerability Reporting separately, then update SECURITY.md and the security form. Establish a separate private conduct-report route. License choice remains pending. Repository administration is not implicitly authorized by a content PR.

# Contributing to AETHER

AETHER is in P0 feasibility. There is no implemented app, supported product version or approved production architecture. Read [AGENTS](AGENTS.md), [SECURITY](SECURITY.md) and [Code of Conduct](CODE_OF_CONDUCT.md).

## Development lifecycle

1. Open or identify an Issue describing the problem, value, scope, exclusions, acceptance criteria, dependencies, security impact and test plan. Obtain scope approval before implementation. Research needs a hypothesis, bounded experiment and evidence. A security incident follows the private reporting policy instead of public reproduction details.
2. Inspect the current repository, main SHA and local changes. Create a short-lived Issue branch from the reviewed baseline; preserve unrelated work. Follow [branch and commit conventions](docs/engineering/git-workflow.md).
3. Record architectural proposals under [decisions](docs/decisions/README.md); no accepted decision without sufficient evidence and explicit approval.
4. Implement focused changes. Do not install or upgrade dependencies outside the approved scope. Follow [coding standards](docs/engineering/coding-standards.md).
5. Run relevant [validation](docs/engineering/testing-policy.md), inspect the diff and sensitive data exposure, and distinguish real, mocked and manual evidence.
6. Open a Draft PR linked to the Issue. State implemented changes, exclusions, security impact, actual validation, deferred checks and recovery. Use `Refs #N` for partial work and `Closes #N` only when merging completes the Issue.
7. Review the latest head SHA. Resolve critical findings, satisfy acceptance criteria and checks, and obtain explicit user merge authorization. A solo maintainer may review using a recorded checklist; do not manufacture independent approval.
8. After an authorized squash merge, verify main's resulting SHA and synchronization. Branch deletion is a separate authorized/settings-controlled action; never delete unmerged work. Do not begin the next task automatically.

## Emergency exception

For a confirmed incident, use a private security advisory or an approved private incident record first. If unavailable, obtain explicit maintainer authorization for a narrowly scoped emergency record and record a sanitized Issue when safe. Keep branch/PR review and explicit merge authorization; this exception does not permit direct main pushes or disclosure of exploit details.

## Review and licensing

Review scope, correctness, failure behavior, privacy, dependency provenance, test evidence and rollback. UI changes need safe screenshots/recordings when relevant. Policy exceptions require explicit documented acceptance, not a silent skipped check. License selection is pending; discuss substantial external contributions before submitting code. Release requirements are in [release policy](docs/engineering/release-policy.md).

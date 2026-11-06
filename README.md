# AETHER

AETHER is a planned native macOS AI assistant for public distribution, initially targeting Apple Silicon. Its mission is natural Korean voice interaction and permission-controlled assistance with local work.

## Status

**P0 — Architecture & Feasibility.** This repository currently contains governance only. Voice, agent execution, computer control, memory, desktop UI and release tooling are not implemented. There is no runnable application or supported application build command. P0-B implementation requires a separately approved Issue and scope.

## Candidate architecture

Tauri 2 with React/TypeScript; Rust host with a small Swift integration layer; TypeScript agent orchestration; OpenAI Responses and Realtime APIs; SQLite; Playwright; Three.js/React Three Fiber; Developer ID signing and notarized DMG distribution. These are candidates, not accepted decisions or verified capabilities. See [architecture](docs/architecture/README.md) and [decision records](docs/decisions/README.md).

## Working with this repository

Read [CONTRIBUTING](CONTRIBUTING.md), [AGENTS](AGENTS.md), and the [Git workflow](docs/engineering/git-workflow.md) before proposing changes. With Python 3 and Git available, run `python3 scripts/validate_governance.py` to check the governance artifacts. It installs nothing and is not an application test.

Report security concerns according to [SECURITY](SECURITY.md); do not disclose vulnerabilities in public Issues. Participation follows the [Code of Conduct](CODE_OF_CONDUCT.md).

No software license has been selected yet; public visibility is not a license grant. License selection remains a maintainer decision before accepting substantial external code or distributing software.

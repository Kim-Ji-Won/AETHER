# Testing policy

## Current checks

Run `python3 scripts/validate_governance.py` from the repository root and `git diff --check`. The validator uses only Python's standard library; JSON-formatted YAML is intentional because JSON is a YAML subset. It checks expected files, local Markdown file links (not anchors or remote availability), Issue form structure, selected workflow safety properties, text hygiene and basic sensitive-file patterns. It is not a complete YAML/schema validator, secret scanner, security proof or application test.

CI currently has one job named `governance`. It runs on PRs targeting main, including edits to PR metadata, and on pushes to main. CI additionally checks PR title/branch/Issue-link conventions; it does not establish the linked Issue's acceptance or approval. No application commands are claimed. Do not make a check required until it exists, succeeds reliably and an authorized ruleset change is reviewed.

## Evidence requirements

Report exact command, version/environment where relevant, exit result and tested SHA. Use PASS, FAIL, BLOCKED or NOT_RUN. Explain failures and deferred checks; an accepted exception must be explicit. Distinguish mock/unit results from signed-build/manual integration results. Do not claim UI/permissions PASS from static tests.

Future check names are plans only: `lint`, `typecheck`, `unit-tests`, `rust-check`, `swift-check`, `desktop-build`, `security-check`. Introduce each with actual implementation and relevant failure tests. Security work needs unauthorized-call, denial/revocation, replay and race-condition checks. Changes should receive proportionate verification, not redundant tests that mirror trivial content.

PR CI uses an ephemeral Linux runner, read-only contents permission, pinned checkout with persisted credentials disabled, no production secrets, no publish step and a five-minute timeout. Never replace it with privileged untrusted `pull_request_target` execution. Review workflow changes as security-sensitive.

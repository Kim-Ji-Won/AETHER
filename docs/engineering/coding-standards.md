# Coding standards

Current scope is governance; no language toolchain or formatter is mandated yet. Use UTF-8, LF, final newlines, descriptive names and focused changes. Markdown should distinguish implemented behavior from proposals. Keep links relative within the repository and avoid personal environment data.

Use Conventional Commits and reviewable boundaries. Do not add generated output, binaries, private data or credentials. Keep dependency lockfiles tracked when a package ecosystem is introduced. Pin deliberate dependency changes and record rationale, compatibility and validation; do not upgrade incidentally.

Future code should expose narrow typed boundaries, validate untrusted input, use explicit cancellation/timeouts and bounded queues, and preserve error context without secrets. Separate UI, policy, execution and persistence. ABI ownership/threading and IPC protocol versioning require documented contracts. Do not build speculative abstractions before evidence justifies them.

Architecture changes require an approved Issue and ADR where consequential. These standards do not authorize implementation of future components.

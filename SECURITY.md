# Security policy

## Reporting route: pending

Private Vulnerability Reporting was observed disabled during P0-G on 2026-11-06. There is no configured private security contact or staffed response team, and no response-time promise. The maintainer should enable GitHub Private Vulnerability Reporting and verify the reporting flow before updating this policy.

Check the repository [Security page](https://github.com/Kim-Ji-Won/AETHER/security). If a private “Report a vulnerability” option is available, use it. If it is unavailable, do not submit exploit details, credentials, logs or personal data publicly. A public Issue may only request establishment of a private reporting channel, without describing the vulnerability. Wait for a verified private route to exchange evidence. GitHub platform abuse should use GitHub's own reporting controls.

Do not publicly disclose exploitable vulnerabilities before coordinating remediation. No released versions are currently supported: no application has shipped.

## Required design boundaries

These are requirements for future implementation, not claims of implemented controls.

- Never commit or log API keys, signing material, credentials, personal files, raw screen captures or private paths. Ignore rules are not a secret scanner. If a secret is exposed, revoke/rotate it using an approved incident process; history cleanup requires separate authorization.
- Prefer local storage and explicit retention/deletion choices. Sending local content to an AI provider is a separate consent boundary. Include derived summaries, vectors, caches and backups in deletion policy. SQLite does not automatically provide encryption at rest.
- macOS permission grants are not per-action consent. Request microphone, screen capture and Accessibility only for explicit tasks, handle denial/revocation, and visibly stop monitoring. Signing/notarization does not bypass permissions.
- The host must validate tool arguments and enforce default deny, least privilege, per-tool authorization and revocation. Bind approval to the actual task, arguments and target; revalidate before execution and record a redacted audit trail. A model's approval field never authorizes an action.
- Isolate untrusted web/file content from policy. Authenticate local IPC, prevent approval replay and duplicate effects, restrict arbitrary execution, and remember that a sidecar is not automatically an OS sandbox.
- Release/update credentials belong outside PR workflows. Validate artifact signatures and recovery before distribution.

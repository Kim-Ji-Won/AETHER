# Release policy

There are no releases, supported application versions, signing pipelines or release credentials configured by P0-G. Release work requires a separately approved Issue and explicit publishing authorization; merge approval does not authorize a release.

Proposed initial distribution is Apple Silicon, Developer ID-signed and notarized DMG. Validate Hardened Runtime, nested code/sidecar signatures, notarization results, stapling, clean-machine installation, permissions, Keychain behavior and signed updates before shipping. These requirements are not evidence of readiness. Mac App Store distribution and entitlements remain unvalidated.

Protect signing/update keys outside the repository and PR workflows. Do not print secrets or expose production environments to untrusted code. Record source SHA, toolchain/lockfile versions, artifact hashes and verification evidence. Keep update signing separate from Apple signing and test tamper rejection, interrupted updates and schema compatibility.

Before publication, review privacy disclosures, retention/deletion, dependency licenses, product license, support/security routes and recovery. Rollback must account for data migrations: a prior binary may not read a new schema. Prefer backward-compatible migrations and tested backups/recovery; never promise automatic rollback without evidence. No automatic publishing, release deletion or history rewrite.

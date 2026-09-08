# Contributor and session guidance

Read README.md, LICENSE-DECISION.md and the owning issue before editing. This is a private bootstrap repository, not a released implementation.

- Preserve authored/source identity and explicit unsupported/incomplete results.
- Use the first specification issue to establish requirements, acceptance fixtures and scoped implementation/CI work before scaffolding speculative crates.
- Inspect existing shared model, contract and Quoin interfaces before duplicating them.
- Record dependency licenses and copied/generated artifact provenance. Do not infer licensing from process or repository boundaries.
- Keep private research, traces and issue links out of public releases. No public posting, visibility change, or source relicensing without explicit approval.
- Use isolated worktrees for concurrent code sessions; preserve unrelated edits.
- Run the actual build/test commands documented in README.md and record their outcomes.
- Owner directive: checks run locally until stable. Every hosted workflow must be manual-dispatch only (`workflow_dispatch`); do not add automatic triggers or dispatch a run without explicit direction. See NFR-002 and IT-004.

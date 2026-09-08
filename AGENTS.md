# Contributor and session guidance

Read README.md, LICENSE-DECISION.md and the owning issue before editing. This is a private bootstrap repository, not a released implementation.

Current ownership (owner clarification, 2026-09-08): `contract-agent-core` is
retired; remaining non-temporal work belongs to the current `quire-agent-a/b/c`
sessions. Route inherited contract/integration claims through `quire-agent-c`
and CO01. The distinct `contract-agent-c` retains `tl-*`. Historical ownership
records do not require another transfer from the retired agent. A keeps the
native compiler/model-contract scope; confirm actual shared-repository claims
with C before overlapping edits.

- Preserve authored/source identity and explicit unsupported/incomplete results.
- Use the first specification issue to establish requirements, acceptance fixtures and scoped implementation/CI work before scaffolding speculative crates.
- Inspect existing shared model, contract and Quoin interfaces before duplicating them.
- Record dependency licenses and copied/generated artifact provenance. Do not infer licensing from process or repository boundaries.
- Keep private research, traces and issue links out of public releases. No public posting, visibility change, or source relicensing without explicit approval.
- Use isolated worktrees for concurrent code sessions; preserve unrelated edits.
- Run the actual build/test commands documented in README.md and record their outcomes.
- Owner directive: checks run locally until stable. Every hosted workflow must be manual-dispatch only (`workflow_dispatch`); do not add automatic triggers or dispatch a run without explicit direction. See NFR-002 and IT-004.

<!-- required-language-policy:start -->
## Required implementation language and containment

Owner directive (2026-09-07): first-party production and possible qualification paths must be **Rust**, including libraries, CLI/tools, generators, validators, canonicalization, result adapters, test assertions and fixture/CI audits. "Glue", prototypes, scripts and tests are not exemptions; a Rust wrapper around non-Rust semantic logic is not remediation.

- Every new TypeScript choice needs explicit owner approval for its exact scope. Any other executable language must be brought to the owner's attention; do not silently introduce or expand it.
- Quoin's existing implementation is explicitly retained for now: **contain its spread, do not schedule a wholesale Quoin rewrite**. Keep new reusable semantics in Rust behind versioned, structured interfaces. Existing Node/TypeScript is not blanket permission for new components or expanded language scope.
- Inventory existing JavaScript/Python/shell and executable generated/inline CI logic; record a Rust remediation owner or an explicit owner disposition. Data schemas and foreign-language fixture samples must be distinguished from executable checks. External tools/hosts/bindings need a scoped disposition, not an invented exemption.
- Reuse shared assurance contracts and Quoin evidence ownership; do not recreate local evidence frameworks. Classify each helper as an existing shared capability, a missing reusable capability, or domain-specific logic before porting.
- Complete /specify and /spec-review for the changed requirements/interfaces, resolve findings, then implement and verify. Preserve ongoing work and actual evidence provenance. Do not claim a language port or qualification complete from a policy update.
- Filament repositories/tickets are excluded from this remediation. Interface consumption does not authorize producer changes.

Before closure: record executable-path inventory, approved dispositions, Rust implementation/test/CI parity and failure-path evidence for the reviewed scope. Existing Quoin retention is the only accommodation recorded here; this block grants no new TypeScript approval.
<!-- required-language-policy:end -->

/specify and /spec-review are mandatory before implementation. Record the owner-selected review set, reviewed revision, actual review artifacts and finding dispositions. Preserve already-written work; pause further unreviewed implementation/merge and record the true sequence.

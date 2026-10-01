---
id: SR-949
title: "QSL-360 code review (with rust-review lane and test-oracle check) of PR 558, DefinitionLock read from QSpec's lock by reference"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@13f0b2a6cf2a50af6768366cb754b0c3995ad71a; diff 3260a231...13f0b2a6; qsl-semantics/src/value/definition.rs; qsl-semantics/src/value/diagnostics_catalog.rs; qsl-semantics/src/value/mod.rs; qsl-semantics/tests/it/complete_value_lock.rs; qsl-package/src/emit.rs; qsl-package/src/emit/tests.rs; qsl-bench/src/text_cluster.rs; Cargo.toml; Cargo.lock; qsl-semantics/Cargo.toml"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: reviews
---
## Summary

Ticket: QSL-360. PR: quire-spec-language#558 at 13f0b2a6, diff `3260a231...13f0b2a6`.

What I checked:

- **The copy is gone.** `CATALOG`, `ALWAYS_ROLES`, `CONDITIONAL_ROLES`,
  `EXACTLY_ONE_ROLES`, `AGENT_IX`, `DRAFT`, `DIGEST_DOMAIN` and the `1-draft.1`
  literal are deleted. `DefinitionLock::read` deserializes QSpec's
  `complete-value-lock.json` (serde, borrowed `&'static str`) and refuses an unknown
  role, a role without exactly one row, a selection trigger outside
  `trigger_vocabulary`, and a refusal-code set other than `SelectionRefusalCode::ALL`
  (duplicates included, through the length check).
- **No stale digest left.** `git grep` at HEAD (excluding spec/reviews history) finds
  none of `1d8b15f8`, `8b8500fa`, `94580e10`, `4d0dcb64`, `5438519e`, `644e325d`,
  `012e66aa`. The only lock digests still typed in code are the root `c8c7ae9f`
  (inside QSL source-text fixtures, `profile ... digest "sha256:..."`, which a test
  program must write) and the text profile `cd4a985a` (see the gap analysis, SR-950).
- **`revision()` 1-draft.1 to 1-draft.2.** `DefinitionLock::revision()` has no caller
  outside the test that now compares it with the document's `revision`. It feeds no
  identity preimage and no wire member, so the change is a correction with no
  package_id effect.
- **Diagnostics catalog.** `native_diagnostics_catalog()` parses identity and
  revision from the header line and hashes the bytes. QSpec publishes no
  machine-readable record of that document's identity or revision (no JSON in
  `proposals/quire-v1/definitions` names it), so the header is the only source. The
  parse takes the first line starting ``Interpretation identity: ` `` and splits on
  the two backtick delimiters: adequate. Authority `agent-ix`, namespace
  `quire-draft` and domain `quire.definition.bytes/v1` are identity labels and a
  protocol constant, not data copied from that document. Accepted.
- **Panics.** `pinned()` and `native_diagnostics_catalog()` panic if the compiled-in
  bytes do not read. The bytes are compile-time constants and
  `the_compiled_in_lock_reads` / `the_native_diagnostics_catalog_reads_its_header`
  read them in the same build, so a broken asset fails the gate, never a user's
  compile. Documented under `# Panics`. Accepted.
- **SelectionRefusalCode and CatalogRole stay typed.** Both are closed vocabularies
  the code dispatches on, and the read checks both against the lock in both
  directions. That is not a copy: the lock is the authority and drift fails the read.
  `Trigger` should have been kept on the same terms (FND-001).
- **Dependency.** `Cargo.lock` adds only `quire-specification-qualification` (git,
  branch `task/std-130-definition-lock-accessor` at `ce802b78`, the current head of
  quire-specification#167) and its edge from `qsl-semantics`. The crate has no
  dependencies. The PR body says not to merge before STD-130 and to switch to
  `branch = "main"` then.
- **Gate.** The coder's `make ci` log ends `head=13f0b2a6... exit=0`. It shows
  `emit::tests::admission_corpus::every_emitted_node_family_is_admitted_at_its_package_id ... ok`,
  `the_lock_selects_the_catalog_definitions ... ok`, the five new
  `complete_value_lock` tests ok, and `string-edge` and
  `arch-lint-canonical-encoder` passing. I did not re-run it.

Rust-review lane: no `unsafe`, no integer conversion, `thiserror` error enum with a
`#[from] serde_json::Error`. `DefinitionLock` losing `Copy` compiles workspace-wide.
`read(bytes: &'static str)` forces a caller with runtime bytes to leak; only
compiled-in bytes and tests call it, so acceptable.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Deleting the `Trigger` enum puts string comparison after the edge. `trigger_set` now resolves a trigger by `*known == *code` and `admit_selection` chooses its refusal by `triggers.contains(trigger)` over `&str`, and `AdmittedSelection::triggers()` returns `BTreeSet<&str>`. ADR-012 §9: "After the edge, no code compares a string to choose behaviour." `xtask string-edge` misses it only because it cannot see a variable-to-variable `&str` comparison (its own doc says so). `Trigger` is a closed vocabulary exactly like `SelectionRefusalCode`, which the PR keeps typed and checks against the lock. Restore `Trigger`, have `read` refuse a `trigger_vocabulary` other than `Trigger::ALL`, and map the selection rules to `Trigger`. | qsl-semantics/src/value/definition.rs:466,572-584,665-669,690 |
| FND-002 | medium | Two of the four refusals the new reader adds have no test: `LockReadError::UnknownTrigger` and `LockReadError::RefusalCodes` (wrong set, or a duplicate code). The "missing" half of `a_lock_with_an_unknown_or_missing_role_does_not_read` is not tested either: renaming `root` to `edition` yields `RoleRowCount { Edition, 2 }`, and no case asserts `count: 0`. Add a mutation per branch (trigger rename in `package_selection`, a refusal code dropped and one duplicated, a row deleted). | qsl-semantics/src/value/definition.rs:398,401; qsl-semantics/tests/it/complete_value_lock.rs:68-87 |
| FND-003 | low | `the_compiled_in_lock_reads` checks each row's role, identity and digest against an independent `serde_json::Value` parse, but not authority, revision namespace and value, digest_domain or artifact_path, and not the `package_selection` rules (`always_roles`, conditional, exactly-one) against the document. Revision value feeds every emitted `DefinitionRef` and the FR-110 match. Compare the whole row and the selection rules. | qsl-semantics/tests/it/complete_value_lock.rs:49-63 |
| FND-004 | low | The `CatalogRole` variant docs still restate each lock row's identity and artifact path (for example `quire.value.accounting/v1` / `value-accounting.md`, `../package-contract.md`). That is the same data the PR removes from `CATALOG`, kept as prose, and it will drift silently when QSpec renames a row. Describe each role by what it is and drop the identity and path. | qsl-semantics/src/value/definition.rs:35-91 |

## Verdict

The main change is correct and does what the ticket asks: the 21-row copy is gone,
the reader is strict, the six stale rows are now current, the diagnostics digest is
computed from real bytes, and the gate is green with the admission corpus passing.
FND-001 is a real ADR-012 §9 regression introduced by the PR and should be fixed in
this PR. FND-002 is a test gap on the new reader. FND-003 and FND-004 are low.

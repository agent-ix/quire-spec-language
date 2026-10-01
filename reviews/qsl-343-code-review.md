---
id: SR-920
title: "QSL-343 code review (with rust-review lane) of PR 548"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@3e95fb6a6f9d749b8fcd815f61d2814d7f41ea27; qsl-package/src/checked_v2.rs; qsl-package/src/checked_v2/tests.rs; qsl-package/src/emit.rs; qsl-package/src/emit/extent_agreement.rs; qsl-package/src/emit/tests.rs; qsl-package/src/lib.rs; tests/it/config_version_spine.rs; quire-contract-model at IR PR 235 (fa68112, read only)"
review_set: subset
---
## Summary

Ticket: QSL-343. PR: quire-spec-language#548 at 3e95fb6a. Methods:
code-review, with the rust-review lane folded in. The gate was not re-run:
the caller's `make ci` log for 3e95fb6a against the IR #235 path patch ends
`head=3e95fb6a... exit=0`.

The diff matches IR #235's interface change. `own_evidence` drops its digest
loop and now carries only the lock's required features. The
`insert_dependency_package` call loses its version argument. The version
is still enforced by QSL, because `ClosureReader::read_emitted` pins each
dependency read at `resolved.selection.version`, and the admitted cache is
keyed by `package_id`. The four `locked_artifacts`/`record_locked_artifacts`
helpers and the `evidence()`/`locator()` fixtures are deleted. No
`insert_artifact_*` or `CheckedArtifactLocator` use remains in the workspace.

QSL's own `stale_dependency` paths are untouched and still tested:
`LinkRefusal::DependencyIdentityMismatch` (ADR-015 D-1 step 5,
`e4_refuses_a_stale_dependency_and_a_conflicting_diamond`), the library
pin `StaleDependency{ByteDigestMismatch|RevisionMismatch}`
(`refuses_when_the_pinned_package_id_disagrees`,
`refuses_when_the_pinned_version_disagrees`), and IR's envelope
`StaleDependency` on a non-recomputing `package_id`
(`refuses_package_id_that_does_not_recompute`). These tests are now more
discriminating, not less. Before, every read carried artifact evidence; now
it carries none, so a `StaleDependency` can only come from the identity
check the test names.

`diagnostics_catalog` is private, and it still has one in-crate use:
`emit.rs:920` writes it as `diagnostics.catalog`. So it is live code and
correctly kept as a private function, not deleted. Nothing else in this
workspace uses it. quire-driver's `handoff.rs` does, which is already routed.

Merge preconditions (not code findings):
- GitHub reports the PR CONFLICTING. origin/main a486e555 (#544, QSL-339)
  touches 9 of the 11 changed files. The FR-093 and TC-416 hunks conflict on
  the "(QSL-260)" lines this PR deletes. Rebase, then run the gate again.
- The branch builds only through the uncommitted `.cargo/config.toml` patch.
  IR #235 must merge first. Then commit the Cargo.lock bump to that IR rev.
- IR #235's head moved to fa68112 after b50cd28. 32e3371 removed
  `CheckedPackageRefusalCause::RevisionMismatch` and the domain-package
  digest-domain check. QSL references neither, but the pre-merge gate should
  run against the IR rev that actually merges.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `read_checked_package_v2`'s doc still says "`evidence` proves the wire's locked sources, definitions and domain packages are current; ... this reader does not itself know which bytes are current". That is the removed staleness behaviour. The module doc (lines 33-38) was updated to "domain package documents, admitted dependency packages and supported features", but this one was not. | qsl-package/src/checked_v2.rs:613-616 |
| FND-002 | low | `own_evidence`'s doc keeps the staleness-era rationale: "It attests this emission only: [`Emission`] is built by [`emit_checked`] alone, so no caller can pair it with other bytes." The evidence is now just the lock's own required features. The pairing guard was for per-artifact digests. IR #235 fa68112 removed the same "attest" wording on its side. Drop the sentence. | qsl-package/src/emit.rs:944-946 |
| FND-003 | low | The edited `read_fixture_wire` doc line runs to 107 columns ("...as supported. The published fixture is pretty-printed; the wire is its"). It was not rewrapped after the clause was removed. | qsl-package/src/checked_v2/tests.rs:1376 |
| FND-004 | low | `refuses_package_id_that_does_not_recompute`'s comment says IR's `StaleDependency` "is the wrong code for a condition-2 recompute failure (filed with IR as a ticket)". IR #235 now documents `StaleDependency` as covering exactly that: "The package ... does not match the content identity it names: the `package_id`, identity preimage...". The comment now contradicts IR's own definition. Line not in the diff, but it is part of the IR #235 alignment this PR does. | qsl-package/src/checked_v2/tests.rs:647-653 |

## Verdict

Correct and complete for the code change. There are four low doc and comment
findings. Two are staleness wording this PR left behind (FND-001, FND-002).
Nothing QSL still needs was deleted. Rust lane: no new panics, unsafe, integer
conversions or public surface. One item is removed from the public API, the
`diagnostics_catalog` re-export, and its only external user is already routed.
Merge needs a rebase onto a486e555, the IR #235 merge plus a Cargo.lock commit,
and the gate run again.

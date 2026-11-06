---
id: SR-20261105101066
title: "Code and Rust review of QSL-1778 native checked temporal reports"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; QSL-1778; PR #2448 commits bf80b2d17..b80fec47d beyond qsl-1777-fr350-runtime-b; src/temporal/owner.rs, src/temporal/owner_codec.rs, src/temporal.rs, src/temporal/protocol.rs, src/temporal/result.rs, qsl-eval/src/temporal/evidence/{call,identity,mod,package,request}.rs, Cargo.toml, tests/it/main.rs, tests/it/native_owner_report.rs, tests/it/family_outcome_layering.rs, FR-052"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-052
    type: reviews
---

## Summary

Ticket: QSL-1778. Code review with the rust-review lane folded in, scoped to the ten commits PR #2448 adds on top of its stacked base `qsl-1777-fr350-runtime-b` (head b80fec47d).

## Verdict

**CHANGES REQUESTED.** One high: the new integration module is not feature-gated although it uses two `checked-handoff`-gated sibling modules, so the default-feature `it` test binary does not compile (reproduced: 23 E0433 errors). Two medium soundness findings in the strict reader, both reproduced with a throwaway probe test: a result identity from a different evaluation is admitted, and a flipped truth tag is admitted. With `--features checked-handoff` all nine PR tests pass.

Sound: the codec's strict framing (fixed member order, length prefix, trailing-byte and truncation refusals, no pre-allocation from untrusted counts); the evaluator replacing caller valuation maps with the admitted relation so caller truth cannot leak into evaluation; predecessor lookup reusing the engine's ledger and ObligationKey; the rename to `ArtifactReport`/`evaluate_artifact_v2`; the layering test updated for the real root to qsl-eval edge.

## Scope

- FR-052-AC-9 (examined): truth, settlement and support equal the FR-043 evaluation; no input permits caller-supplied truth, settlement or support to determine that result.
- FR-052-AC-12 (examined): correction predecessor rule on the native Report route.
- FR-052-AC-15 (examined): report subject, observation, truth, settlement, support and outcome equal those of the evaluation.
- FR-052-AC-16 (examined): support entries name admitted trace positions and rows with multiplicity.
- FR-052-AC-17 (examined): round-trip equality and byte determinism across calls and builds.
- FR-052-AC-18 (examined): each single mutation of owner bytes is refused with the named ReportRefusal.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `mod native_owner_report` is ungated but uses `checked-handoff`-gated `temporal_package_route` and `checked_views`; default-feature `it` binary fails to compile (23 E0433), breaking ci-default-features clippy/test | tests/it/main.rs:123 |
| FND-002 | medium | Strict reader admits any well-formed result identity not equal to the predecessor's input; an identity from a different evaluation is admitted, though its Clause/Occurrence/Relation members are checkable against the subject and predecessor | src/temporal/owner.rs:413-435 |
| FND-003 | medium | Reader admits outcome/truth/settlement tags unchecked against anything; flipping True to False in valid bytes yields Ok(Some(False)), so caller bytes determine a Report's truth (tension with FR-052-AC-9) | src/temporal/owner.rs:372-381 |
| FND-004 | low | `encoded()` maps a serialization fault to `ReportRefusal::UnknownTag`, while `row_key` panics via `expect` for the same fault class | src/temporal/owner.rs:143-145 |
| FND-005 | low | Package version is a hard-coded literal on both writer and reader, so `BindingDimension::PackageVersion` can never mismatch; binding is a positional `Vec<Vec<u8>>` zipped with `1..=5` that would truncate silently | src/temporal/owner.rs:86-97 |

## Evidence and limits

Built only through locked-build.sh with the shared target, jobs 4, debug 0. `cargo check --test it` (default features) fails with 23 E0433 in tests/it/native_owner_report.rs. `cargo test --features checked-handoff --test it native_owner_report`: 9 PR tests pass. A local, uncommitted probe test (removed) confirmed FND-002 (`foreign identity admitted: true`) and FND-003 (`forged truth admitted: Ok(Some(False))`). No full gate run.

## Dispositions

Round 1, current head `5ed40a131`. Original review rows unchanged.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Feature gating now matches the two required sibling modules; static inspection, no new compile. |
| FND-002 | fixed | Shared evidence-owner canonical encoder validates all identity members against actual request, observation and checked subject. |
| FND-003 | rejected | FR-052 evaluation-input prohibition does not require a non-evaluating decoder to authenticate a known truth tag; see full reason in comment. |
| FND-004 | fixed | The same fallible encoded helper is used throughout row_key; no serialization expect remains. Closed checked identity/role values expose no caller-supplied serializer, and the declared closed refusal enum is retained. |
| FND-005 | fixed | Fixed-size array prevents silent zip truncation; PACKAGE_VERSION is one constant. Typed CheckedPackageV2 admits only v2, so no dynamic alternative package version exists; byte-version mismatch remains meaningfully refused. |

### Round 2

Reviewed head `1ad7a1fc0`.

No defect findings were still open in this artifact before round 2; no new findings. Prior fixed/rejected outcomes remain unchanged.

## Disposition evidence

Static review in the sole existing ticket worktree, no build/clone/worktree created. Existing coder receipts considered but not represented as reviewer reruns. Final gate remains the lead's next stage after every open finding is resolved.

## Round 2 evidence and readiness

Disposition round 2, final reviewed head 1ad7a1fc0 (owned fix 402547d2a..1ad7a1fc0). Same existing QSL-1778 worktree only; no source edits, build, clone or extra worktree. Relevant merged-main delta leaves native owner production behavior and canonical identity validation unchanged; owned production delta is formatting only. Both new test attributes now pair TC-144 with their criterion ids. The explicitly ignored local lane cargo-compiles the integration executable twice with different Rust metadata in the sole existing target, asserts fresh=false for each compilation, executes the exact fixture in each binary, and compares nonempty byte outputs. Existing pre-rebase coder receipts show eleven normal tests passing and the two-build lane passing with 7209 bytes from each binary; these were inspected, not rerun. Foreign-clause control uses a genuinely admitted other package, and the shortened request rejects its still-admitted position-1 valuation row.

Source-map reachability independently traced: quire-contract-model/src/checked_package/common.rs validate_source_map_entries constructs expected (node, role, ordinal) from all graph occurrences and compares actual source-map keys exactly, returning InvalidSourceMap when actual != expected. checked_package/v2/mod.rs calls that validator before constructing the admitted CheckedPackageV2, whose fields are private; TemporalSubjectView::arena_of requires a declared occurrence. Thus a missing map entry for an accepted occurrence cannot reach CheckedSubject::new through the public admitted API. TC-144 now documents this defensive-arm proof rather than fabricating an invalid checked package.

All findings across the six reviewed artifacts have a latest resolved outcome. No new fix-diff findings. Review mergeable, subject to the lead's one final gate and normal integration checks; no claim that the unavailable repository-wide matrix or final gate passed.

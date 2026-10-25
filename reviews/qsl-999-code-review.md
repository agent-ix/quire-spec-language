---
id: SR-17120
title: "QSL-999 code and Rust review of the load-buffering warning and manifest memory_models routing (quire-spec-language PR 1495)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@8244b0dcb06c2b63963913db22d19855cef5ce45; git diff origin/main...HEAD (72 files); genuine change: qsl-eval/src/simulation/protocol/access.rs; qsl-eval/src/simulation/protocol.rs; qsl-replay/src/model_check/settle.rs; qsl-replay/src/lib.rs; qsl-route/src/lib.rs; qsl-analyze/src/model_check/manifest.rs; 16 call-site and test files"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-226
    type: reviews
---

## Summary

Ticket: QSL-999 (PR quire-spec-language#1495). `/code-review` with the
`/rust-review` lane over `git diff origin/main...HEAD` at `8244b0dcb`.

**The PR commit reverts merged work on main.** The reflog shows a `wip`
commit on `579d4f401` (QSL-814, #1480), then `reset: moving to origin/main`
(`a0403cc06`), then one commit of the whole working tree. The result: 50 of
the 72 files in the diff have a HEAD blob identical to `579d4f401`'s, so the PR
undoes main's changes to them from #1482, #1485, #1486, #1487, #1488 and #1470.
For example, it deletes `qsl-replay/src/model_check/subject_identity.rs` and
its `SubjectIdentity` export (#1488), the FR-177-AC-8..12 tests in
`qsl-analyze/tests/it/hyper_witness.rs` (#1485), the ModelCheckLimit
snake_case names in `qsl-replay/src/model_check/limits.rs` (#1486), the
refinement precedence fix (#1487), and IR-1007's FR-485 and TC-1263 spec files
and reviews (#1470). The work the ticket actually asks for is
`git diff 579d4f401 8244b0dcb`: 22 files, +643 -98. That diff is what the rest
of this review judges.

Checked and found sound in the genuine change:

- `ManifestMembers`: there are 22 `descriptor_from_manifest` call sites on main
  and 23 at HEAD (one new test). Every converted site maps the old positional
  `(domains, fairness_kinds, evidence_kinds)` arguments to the same-named fields
  in the same order, with `memory_models: None` everywhere except EN-1's
  `manifest::descriptor`. Nothing is dropped or reordered. No `#[allow]` was
  added: the only `clippy::too_many_arguments` allow, on
  `descriptor_from_manifest_with_dynamics`, is pre-existing.
- The `ra` gating (`governing` per `parallel`, inherited by nested ones), the
  same-location exclusion, the exclusion of locations only `t` touches
  (`location: None`), and the treatment of an acquire, acq_rel or seq_cst fence
  as an acquire fence.
- A non-atomic load (`ordering: None`) is treated as relaxed. A reviewer probe
  shows it warns.
- No new production `unwrap`, `expect`, `panic!` or indexing. The only
  qsl-eval `simulation/protocol/*` hunks are the new function and its
  re-export. `ra.rs` is untouched. The genuine diff adds no string-edge
  candidate.

Reviewer probes: an extra test file in a private copy at HEAD, never in the
worktree. Mutations:

- (M1) Dropping `loaded != location` makes the AC-1 test fail on the
  same-location variant.
- (M2) A fence that never clears pending makes it fail on the acquire-fence
  variant.

Both mutations were caught.

## Verdict

FAIL. FND-001 alone blocks merge: merging reverts six merged PRs. The genuine
change has real false negatives and false positives in `load_buffering_shapes`
on control constructs beyond a straight-line `sequence`, and the
`memory_models` member skips the refusals that FR-331-AC-30 states.

## Findings

| ID | Severity | Summary | Refs |
|----|----------|---------|------|
| FND-001 | high | Commit 8244b0dcb was made by resetting a stale wip tree (base 579d4f401) onto origin/main, so 50 files revert main's #1482/#1485/#1486/#1487/#1488/#1470 changes (subject_identity.rs deleted, FR-177-AC-8..12 tests, IR-1007 FR-485/TC-1263, limit names, refinement fix); rebuild the branch from origin/main with only `git diff 579d4f401 8244b0dcb` | qsl-replay/src/model_check/subject_identity.rs; qsl-replay/src/lib.rs:136; qsl-analyze/tests/it/hyper_witness.rs |
| FND-002 | medium | Any attempt whose ordering is acquire, acq_rel or seq_cst clears the pending loads, a store or local access included; FR-226 says only an acquire or seq_cst load (or acquire fence) does. Probe: `Lx relaxed; Sz seq_cst (z t-local); Sy relaxed` reports 0 shapes; spec wants (Lx, Sy) | qsl-eval/src/simulation/protocol/access.rs:1175-1201 |
| FND-003 | medium | `choice` starts `after` from the pre-choice `pending`, so loads survive even when every case runs an acquire load or acquire fence; this is a false-positive warning on a branch with no load-buffering path (probe A: 1 warning, spec 0) | qsl-eval/src/simulation/protocol/access.rs:1136-1142 |
| FND-004 | medium | `repeat` is walked as body;exhausted;body;exhausted and `await` (the `_` arm) as event;then;timeout: no zero-iteration or body-then-exit path, and `then`/`timeout` treated as sequential. An acquire in a repeat body or in `then` hides a shape on the zero-iteration/timeout path (false negative; probe C, `while {false}`: 0 warnings, spec 1); a load in `exhausted` or `then` pairs with a store in body/`timeout` (false positive) | qsl-eval/src/simulation/protocol/access.rs:1143-1147; qsl-eval/src/simulation/protocol/program.rs:59-74 |
| FND-005 | medium | `descriptor_from_manifest` admits `memory_models` without FR-331-AC-30's refusals: `Some(&[])` is admitted as sc-only, repeated models are silently deduplicated, and the member on a non-temporal descriptor is silently dropped, unlike `fairness_kinds`/`evidence_kinds`/`dynamics`; the doc comment cites FR-331-AC-30 | qsl-route/src/lib.rs:319-323,375-379 |
| FND-006 | medium | The AC-1 test compares the warned record with a different package (load `acquire`), not the same request settled without the warning, and asserts no obligation identity at all, so two AC-1 clauses are approximated or missing | qsl-analyze/tests/it/load_buffering.rs:91-107 |
| FND-007 | low | `load_buffering_warnings` turns every failure into no warnings (missing protocol, `ModelSystem::new` error, any `ProtocolRefusal` incl. internal faults via `unwrap_or_default`); the comment justifies only the access-failure case, which the item settles itself | qsl-replay/src/model_check/settle.rs:322-346 |
| FND-008 | low | `descriptor_from_manifest_with_dynamics` hard-codes `memory_models: None`, so a temporal provider that carries `dynamics` can never advertise a weak model although QSpec FR-331 allows both members | qsl-route/src/lib.rs:426-437 |
| FND-009 | low | Two converted call sites in evidence_kinds.rs place a comment between a field name and its value; rustfmt gives up on them and leaves one-space indentation | qsl-route/tests/it/evidence_kinds.rs:32-39,402-408 |

## Coverage

- FND-001 (examined): every file in `git diff --name-only origin/main...HEAD`.
  Each HEAD blob was compared with `579d4f401`, with the wip commit
  `05b011077` and with `origin/main`. There are 50 pure reverts and 22 genuine
  files. `qsl-replay/src/lib.rs` carries both a revert and a genuine hunk.
- Gates run on a private copy of HEAD:
  - `cargo test -p qsl-analyze --test it -- load_buffering en1_advertises_the_memory`: 2 passed.
  - `cargo test -p qsl-route --test it -- memory_models`: passed.
  - Mutations M1 and M2 were both caught by the AC-1 test.
  - The full workspace clippy and `make string-edge` were not re-run here.
- Probes on HEAD:
  - B (seq_cst store between): 0 warnings, against 1 expected by the spec.
  - D (relaxed RMW on shared `y` after a relaxed load of `x`): 0 warnings;
    this needs a spec ruling.
  - E (non-atomic load): 1 warning.
  - F (baseline): 1 warning.
  - A (`Lx relaxed; choice { case yes when {true}: fence acquire; case no
    when {false}: fence acquire }; Sy`): 1 warning, against 0 expected by the
    spec. This confirms FND-003.
  - C (`Lx relaxed; repeat max 2 while {false} { Lz acquire } exhausted {};
    Sy`, so zero iterations): 0 warnings, against 1 expected by the spec. This
    confirms FND-004.
  - The `await` half of FND-004 was found by reading the code only.

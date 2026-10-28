---
id: SR-1723002
title: "gap analysis of PR #1723 (QSL-1181 slice 3) against FR-375-AC-11/12/13/19/22/27/28, FR-322-AC-87/88/128/129, FR-464-AC-14"
type: SpecReview
analysis: gap-analysis
date: 2026-10-28
scope: "agent-ix/quire-spec-language; PR #1723 (0c943010a); QSpec 48373530 FR-375-AC-11, AC-12, AC-13, AC-19, AC-22, AC-27, AC-28; FR-322-AC-87, AC-88, AC-128, AC-129; FR-464-AC-14; FR-432 as amended by #471"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: reviews
---
# SR-1723002: gap analysis of PR #1723 (QSL-1181 slice 3)

## Summary

Ticket: QSL-1181. Plan completion: not assessed.

| Requirement | Measured |
| --- | --- |
| FR-375-AC-11 | Met for a local clause: `protocol` is the clause, it is a local dependency, and the node id differs from the model subject's. A clause from a dependency package is not reachable: the lookup is local, so such a refinement is not emitted. |
| FR-375-AC-12 / AC-13 | **Built but untested** (FND-001). |
| FR-375-AC-27 | Unchanged (slice 2). |
| FR-375-AC-28 | Met for the catalog: effect and commit are separate event-kind classes with binder records, and `cend` is on the template with empty `binder_types`. The `cend` assertion cannot fail today (FND-003). |
| FR-375-AC-19, AC-22; FR-322-AC-87, AC-88, AC-128, AC-129 | **Not in this PR** (FND-002). The gate still requires empty `steps`, `objects` and `populations`, so no step rows, object or population rows, or `refinement_step_rows` sidecar are emitted, and a mapped refinement refuses `RefinementNotEmitted`. |
| FR-464-AC-14 | IR-side (reader ordering). Not testable until the IR-874 read-back is un-ignored. |

**Can `abstract_internal` be tested with the flag on? Yes, in the default build.**
- The PR body says the support flag "is off by default (FR-135-AC-9 refuses it)". That is wrong: `RefinementSupport::BUILD`, the default, has `refinement_internal_list: true`.
- My probe (scratchpad probe_1723.diff) took `protocol_subject()` with `.with_refinement_internals(json!([{"refinement":"R","internal":["A::Local::Done"]}]))` and the default check. It emits `abstract_internal = ["Local::Done"]`, and the node id differs from the run without the list.
- Its read-back is refused at the same IR-874 B3 point (`class_catalog/abstract/0`).
- `A::Local::duplicate` and `A::Local::lose A::Local::ch` are list defects, as expected.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | **`abstract_internal` emission is untested, though it is testable.** The default build admits a non-empty internal list, and my probe emits `["Local::Done"]` with a different node id. Add a test: one for FR-375-AC-12 (the list present on the body; the value absent, `[]`, or naming another refinement gives no member) and one for FR-375-AC-13 (a different id from no list; reordering the value's entries changes nothing). Correct the PR body's statement about the flag. | qsl-semantics/src/check/refinement.rs:579-589; qsl-semantics/src/check/refinement.rs:170-175 |
| FND-002 | low | **The deferred row slice is recorded nowhere.** Step rows, object and population rows, and the `refinement_step_rows` sidecar (FR-375-AC-19 and AC-22, FR-322-AC-87, AC-88, AC-128, AC-129) are not in this PR. Only a doc comment says "Later slices admit mapped subjects". The PR body's "Not covered" does not list them, and QSL-1181 has no slice plan comment and no child ticket for them (its only child is QSL-1186, protocol controls). Name the slice that carries them in the PR body and on QSL-1181. | qsl-semantics/src/check/refinement.rs:655-670 |
| FND-003 | low | **The `cend` assertion is vacuous today.** A mutant that gives `cend` the template's `binder_types` still passes `effect_commit_and_cend_classes_are_catalogued`, because every template's record is empty, as the PR body says. Nothing fails until a template has a record. Note it in the test, or use a template with a binder record once the language allows one. | qsl-semantics/src/check/refinement/protocol/tests.rs:1955-1993 |

## Verdict

**Not mergeable until FND-001 is fixed** (a small test). FND-002 and FND-003 may be fixed in the same round, or deferred with a stated location.

Gates at 0c943010a (locked-build, jobs 4, `--all-features`, debug info off, QSPEC_DIR=~/dev/quire-specification 48373530):
- fmt: exit 0.
- qsl-semantics `refinement`/`protocol`/`catalog`: lib 177, it 98, all passing.
- qsl-package `refinement`/`catalog`/`protocol`/`owner`/`activate`/`third`: 57 pass, 2 ignored (IR-874).
- `cargo clippy -p qsl-semantics -p qsl-package --all-targets -D warnings`: exit 0.

## Dispositions

Round 1 (head 950c18266).

Round 1. Fix 0c943010a..950c18266 (tests and test docs only, plus the reviews/ SR files).

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | AC-12 and AC-13 are now tested, and the flag claim is corrected. Details below. |
| FND-002 | fixed | The PR body names slice 3b (`qsl-1181d-rows`, stacked on this PR) for the rows and the sidecar, and QSL-1181 comment f619bbbc records it with the full slice order. |
| FND-003 | deferred | The cause is a spec gap: FR-432's binder-record table defines no record for a template's `cattempt`, so no template can carry a non-empty record. It is filed as STD-493 comment f8f915a0, gap 2, and the test doc now says the assertion bites once a template has a record. Correct as a deferral. |

**FND-001 in detail.**
- **AC-12:** `the_internal_list_value_hosts_abstract_internal` covers it: `R` writes `["Local::Done"]`; the value absent, `[]`, or naming only `R2` writes no member.
- **AC-13:** `the_internal_list_enters_the_identity_and_its_order_does_not` covers it:
  - the list changes `R`'s node id and the package id;
  - reordering the value's objects or entries changes neither;
  - renaming the alias `A` to `S`, with the spellings renamed to match, changes neither.
- **Mutant:** dropping the check-time `classes.sort()` fails the AC-13 test.
- **PR body:** it now says the default build admits an internal list.
- **Not covered:** AC-13's obligation-identity clause (an item citing the node) is not asserted. The node id it derives from is, so this is not a finding.

**Verdict: mergeable once #1711 lands with IR #627.** No finding remains open on SR-1723001 or SR-1723002.

Gates at 950c18266 (locked-build, jobs 4, `--all-features`, debug info off, QSPEC_DIR=~/dev/quire-specification):
- fmt: exit 0.
- qsl-package `refinement`/`catalog`/`protocol`/`owner`/`activate`/`third`/`internal_list`: 59 pass, 2 ignored (IR-874).
- qsl-semantics `effect_commit_and_cend`: 1/1.
- `cargo clippy -p qsl-semantics -p qsl-package --all-targets -D warnings`: exit 0.


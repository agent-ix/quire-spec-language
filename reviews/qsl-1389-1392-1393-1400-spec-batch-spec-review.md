---
id: SR-2026111015000000000001
title: "Spec batch QSL-1389/1392/1393/1400 and STD-628 QSL side: spec review"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; branch spec-batch-1389-1392-1393-1400 at 9f1fd380075cc61be91c284e6c9afb1b29632848; diff origin/main..HEAD: FR-062, FR-390 (Source-declared operation calls, AC-70..73, Dependencies), FR-495 (AC-9, Dependencies), FR-517, FR-531 (identities/4), TC-1284, TC-1407"
review_set: subset
review_date: "2026-11-10"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-390
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-495
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-517
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-531
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1284
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1407
    type: references
---

## Summary

REVISE. The QSL-1389, QSL-1392 and QSL-1393 parts hold up. FR-495-AC-9 agrees
with QSpec TC-192 DIV-18 and FR-149, and the FR-495 Dependencies pointer to
FR-497 covers U07, U08, U16 to U18 and U29. The FR-062 sentence for STD-628 is
correct: FR-495 defines a package-level `CheckMode::Kernel`, and Linked
admission is unchanged. The identities/4 bump is consistent across the spec.
Nothing is cut: QSL-1400 is specified as supported, with a dependency on IR.

The QSL-1400 section has one high defect. It says a slot expression "calls" an
operation, but it never says which construct reaches the operation anchor.
Fixture E has no slot that calls `step`: per QSL-1400, `step` appears only in
the protocol's `by Service on M::Node::step` anchors. As a result, AC-71,
AC-73 and TC-1407 steps 2 and 4 cannot be built as written, and AC-73
conflicts with the merged rule that view creation is package-wide.

## Review set

`git diff origin/main..HEAD`, 7 files. The checks were the base spec-review
checklist (EARS, atomicity, consistency and cross-references) plus a
correctness read against the merged text and the code on main.
`make check-spec-validation` with quire 0.38.3 exits 0, with no warning on a
changed file. New IDs TC-1407, FR-390-AC-70 to AC-73 and FR-495-AC-9 appear on
no other remote branch: `git grep -w` over all 2447 `refs/remotes/origin/*`
finds them only on this branch, and the 8 open PRs all have origin heads.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The new section opens with "A slot expression MAY call an operation", but it never says which QSL construct is that call in a protocol slot. No merged FR gives a slot-expression form that calls a model operation. TC-1070 states that guard, relation and predicate slots bar model state. QSL-1400 records that `M::Node::step` appears only in `by Service on M::Node::step` protocol anchors, never in a slot call. So AC-71's "a model-owned call in fixture E gives the same result as before" has nothing to refer to. TC-1407 step 2 ("call `Node::step` from a slot of E-S with TC-1070's row-1 supply") names a slot (`Refunds.activation.guard`, body `opened.ready`) that calls nothing, and its supply holds no `Node`. Fix: name the construct that reaches a source-declared anchor (for example the `by Role on T::op` binding, or a dispatch call), and give E-S's exact slot text and supplies. | spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md:649; spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md:958; spec/test-cases/TC-1407-evaluate-a-source-declared-operation-call-from-the-reader-view.md:28 |
| FND-002 | medium | AC-73 says that without the IR witness kind, only the calling slot reports `ViewFailure::Unsupported`/`MissingSelectedJoin` at its `Node` locus, and "no other slot of E-S ... is affected". The merged text says "view creation is package-wide, this refusal leaves no view of the package, as every other unsupported projection does". If the anchor is reached through the protocol's `on M::Node::step` binding, which is projected at view creation, then the whole E-S view fails. Fix: state whether this refusal happens at view creation (package-wide, so drop "no other slot is affected") or at `read_node` of that slot (then say it is an exception to line 631). | spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md:960; spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md:631 |
| FND-003 | medium | No precedence is stated between the witness refusals of AC-72 and the interim refusal of AC-73. R1 (row omitted) gives `MissingIdentityWitness` at the `Package` locus. With no IR kind in the lock, the same call gives `MissingSelectedJoin` at the `Node` locus. TC-1407 step 3 does not say whether the IR kind is present. A third input also has two possible answers: R3 (another emission under another source authority) fits both `ForeignIdentityWitness` and the `source_authority`-disagrees `UnknownIdentityWitness` rule. Fix: state that the QSL row joins run before the IR projection whether or not the IR kind exists, and that Foreign takes precedence over Unknown, as FR-531-AC-5 does. | spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md:685; spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md:959 |
| FND-004 | medium | identities/4 is a breaking format bump, and no code on main has been updated for it yet. This is expected for a spec-first PR, but the coding ticket for QSL-1400 must list these places: `qsl-package/src/bare.rs:32` (`IDENTITIES_FORMAT` = `.../identities/3`, which the reader enforces at :665 and :896 and the writer emits at :1137); `qsl-package/src/bare/carried.rs:2` (doc comment); `qsl-package/src/emit/tests/source_record_rows.rs:325` and `tests/it/bare_package.rs:319` (assert `/3`); and `qsl-package/src/emit/tests/source_union_rows.rs:68`, which covers the `/2` refusal only, so a `/3` refusal test is needed. The repo has no golden `identities.json` fixture, and no other `~/dev` checkout names the format string. | spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md:236; qsl-package/src/bare.rs:32 |
| FND-005 | medium | STD-628 depends on code that is not on main yet. `PackageDeclarations::check` on main takes no `CheckMode`; only the expression-level checks do (`qsl-semantics/src/check/mod.rs:1783`). Open PR #2630 adds the mode. The checker also records no nonzero obligation for an integer `div`/`rem`/`mod` under the default mode: `check/facts.rs` has no `IntegerDivision` arm, so FR-495-AC-3 is unimplemented, and PR #2630's body notes the same. That gap is why `qsl-analyze/tests/it/value_validity.rs:783` and `:809` pass VV-05 and VV-08 today under a plain `check`. Once FR-495-AC-3 lands, those two vectors must use a package-level Kernel check, or they will turn red. Record this on QSL-1393 and QSL-1200. | spec/functional/FR-062-implement-checked-family-contract.md:259; qsl-analyze/tests/it/value_validity.rs:401 |
| FND-006 | low | The FR-062 sentence says Kernel "records no range or nonzero obligation for an integer `div`, `rem` or `mod`". FR-562-AC-6 already leaves an addition's result bound to the kernel under Kernel, and PR #2630 leaves every unproved result bound, nonzero, presence, rational-range and non-empty-reduction obligation to it. The sentence is true but reads as an exhaustive list. Its "a narrowing operand that leaves its range is refused at admission" also needs scoping. Fix: say "records none of the obligations FR-495 leaves to the kernel, among them integer `div`, `rem` and `mod`", and scope the narrowing refusal to `Linked`. This also exposes an existing FR-495 vs FR-562-AC-6 gap in which obligations Kernel drops. | spec/functional/FR-062-implement-checked-family-contract.md:259 |
| FND-007 | low | AC-73 describes temporary behaviour and says its test "is replaced by AC-71's when IR lands the kind". An acceptance criterion should describe the system, not the build order. Fix: keep the interim refusal as Behavior text tied to the IR dependency, and remove "the test is replaced" from the criterion. | spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md:960 |

## Checks that passed

- FR-495-AC-9: `Rational[0, 7; 1, 3]` matches DIV-18's expected type. Promotion
  by QSpec FR-149 under an expected rational type agrees with AC-6's
  complete-value clause. TC-1284 scope, step and tag lines are updated.
- The FR-495 Dependencies rows U07, U08, U16 to U18 and U29 are all present in
  FR-497 and TC-1287.
- identities/4: FR-517:95, FR-517-AC-23, FR-531:236, FR-531-AC-8, the "All six
  members" count (format, declarations, fields, source_records, source_unions,
  source_operations), and the `/3` rejection agree. No spec file still names
  `/3`; only historical `reviews/` files do, and that is correct.
- No ruling cuts a feature. QSL-1400 is specified as supported; AC-73 is the
  interim state of a dependency on IR (IR-1213, QSL-1398).

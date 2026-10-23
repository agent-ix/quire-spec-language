---
id: SR-16324
title: "spec-review of FR-317 bare enum members (quire-spec-language PR 1335)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@8d1321ad; spec/functional/FR-317-resolve-member-references-and-check-union-construction.md; spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md; spec/test-cases/TC-819-s3-checks-union-construction-and-refuses-fr-143-s-four-construction-er.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-317
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-819
    type: references
---

# Spec review of FR-317 bare enum members (quire-spec-language PR 1335)

## Summary

Ticket: STD-376. One spec-review pass over `git diff origin/main...8d1321ad`, checked for the no-drop/no-narrowing/no-deferral rule, testable vectors (key vectors derived by hand), id uniqueness across all open PR heads, cross-repo names, mergeability and gates.

Gate: `QSPEC_DIR=<QSpec origin/main 0eef131f> make check-spec-validation`: exit 0, 19 warnings (none in the touched files).

Examined, clean:

- The rule follows STD-376's "exact missing statements" and resolves the Game fixture of ADR-022 7.2, FR-165, FR-168-AC-2 and FR-169-AC-9 (the uses I checked there are operands of `=`/`!=` against a typed field). Both spellings give one checked term and one node id, as the AC states.
- AC-5 by hand: `Lost != x.phase` is the commuted operand form of `x.phase != Lost`; `x.phase = Draw` names no case of `Phase`, so `missing-name` at `Draw`; `let Lost = 1 in x.phase != Lost` has `Lost` both a case and a binder, so `ambiguous-name`.
- No deferral language. Ids: FR-317-AC-5 appears on no other open PR head.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | Two AC-5 vectors do not fix their own result. `x.phase < Mid`: ADR-022 7.2 declares `Phase` as a plain `enum`, so `<` is already ill-typed on it, and the AC does not say whether `ill_typed` or `missing-name` is reported first. `Lost = Won`: neither operand is a typed non-bare name, so by the rule both `Lost` and `Won` resolve as ordinary names and both are `missing-name`, but the AC and TC-819 step 5 name one refusal (at `Lost`). Fix: use an `ordered enum` for the ordering vector, and state both refusals (or that the first only is reported) for `Lost = Won`. | spec/functional/FR-317-resolve-member-references-and-check-union-construction.md:104 | missing-requirement |
| FND-002 | medium | The exclusion of the ordering operators cites FR-104, but FR-104 states "supply no expected type" for integer coercion between `Integer` and `Int[..]` operands, not for enum members. FR-091 admits `ordered enum`, whose natural use is `status >= DONE`. As written a bare member is refused in an ordered-enum comparison with no stated reason and no vector over an ordered enum. Fix: state the rationale in FR-317 itself, or admit bare members as orderings' operands when the other operand is a typed enum, and add an ordered-enum vector. | spec/functional/FR-317-resolve-member-references-and-check-union-construction.md:68-71 | wrong-requirement |

## Verdict

Mergeable after FND-001, FND-002 (medium) are fixed or dispositioned by the lead.

Conflicts with main: none. FR-091 and FR-093 AC-row and paragraph conflicts with open #1351, #1355, #1357, #1360, #1369, #1374 and #1379 (FR-093/FR-091 regions); clean with #1333 and #1347.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | low | The new unordered-enum sentence cites "QSpec FR-141-AC-5, as FR-360 states for an unordered type". FR-141-AC-5 says only `ill_typed`, and FR-360 is the infinite-trace result disposition; it says nothing about unordered types or `operator-ineligible`. The cause `operator-ineligible` is QSL's own choice, consistent with FR-093's existing rule for orderings over these types. Fix: drop the FR-360 citation and cite FR-093 for the cause. | spec/functional/FR-317-resolve-member-references-and-check-union-construction.md:68-80 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Fix 2c425982. AC-5 now uses the unordered `Phase`: `x.phase < Mid` refuses `ill_typed`/`operator-ineligible` once with no `missing-name`, and `Lost = Won` refuses `missing-name` twice, at `Lost` then `Won`. Hand-checked against the new "Each unresolved bare name is refused on its own" bullet and the unordered-enum comparison rule. |
| FND-002 | fixed | The ordering operators now supply the expected type when the other operand is typed by an `ordered enum`; AC-5 gains an ordered `Level` fixture: `l >= Mid` equals `l >= Level::Mid` with one node id, `Mid <= l` checks the same, `l > Top` refuses `missing-name`. The FR-104 citation is gone. Hand-checked: `Low < Mid < High` by declaration order, `Mid` names no value in scope. |

Round 1 verdict at 2c425982: Mergeable: FND-001 and FND-002 are fixed. FND-003 (low) is open and does not block. Gate: `make check-spec-validation` with QSpec origin/main: exit 0, 19 warnings, none in touched files. The ordered-enum capability is a lead default; its vectors hold by hand.

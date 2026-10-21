---
id: SR-13722
title: "Gap analysis of PR #1169: FR-397 acceptance criteria against TC-1091 and the current checker"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@d8477bd126f11395bd4ad28120bcdcd063bd5908; PR #1169: FR-397-AC-1 to FR-397-AC-6 against TC-1091, ticket QSL-948's acceptance list, FR-093-AC-20 and a scratch compile of every AC source through qsl_replay::compile_admitted"
review_set: subset
date: "2026-10-21"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-397
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1091
    type: reviews
---
# SR-13722: Gap analysis of PR #1169

## Summary

Ticket: QSL-948. Plan completion: not assessed. This PR changes only the spec,
so no code test is tagged FR-397 or TC-1091 yet; the code lane writes them.
This analysis checks that each AC binds to a TC-1091 step, that the ACs cover
the ticket's acceptance list, and how each AC source behaves today. A scratch
copy of the PR head ran a probe test through `compile_admitted`
(`cargo test -p qsl-replay --test rv1169_probe`, outside the PR worktree):

| Source | Today |
| --- | --- |
| AC-1 `add`, Float64 and Float32 | check refusal `IeeeProfileNotAdmitted` at `a + b` |
| AC-2 `keep` | compiles |
| AC-2 `one` (`float64(bits: ..)`) | S2 refusal `UnrepresentedConstruct { production: FloatValue }` |
| AC-3 `to_r` | check refusal `IeeeProfileNotAdmitted` at the convert |
| AC-4 header `profile w`, with `add` and with `keep` | profile refusal `WrongSelectionRole` |
| AC-5 `=` `!=` `<` `<=` `>` `>=` | `IllTyped(OperatorIneligible)` at the operator, six times |
| AC-5 Float32 + Float64 (return Float64 or Float32) | `IllTyped(TypeMismatch)` |
| extra: `quire::value::ieee::numericEqual(a, b)` | `MissingName` |
| extra: `a * b`, and unmoded `Float64` add | `IeeeProfileNotAdmitted` |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Ticket acceptance 2 requires that the Float-to-Rational `convert` lower to `quire.op.ieee.to_rational` and that strict IR admit it. FR-397-AC-3 and TC-1091 step 3 check only the lowering and the selection, and FR-093-AC-20 checks strict-IR admission only for a Float64 addition. Nothing requires IR's v2 reader to admit the `to_rational` package or the Float32 add package. Add strict-IR admission to AC-3 (and to AC-1 for Float32), with matching TC-1091 expected results. | FR-397-AC-3; spec/test-cases/TC-1091-a-package-that-holds-an-ieee-operation-selects-the-ieee-profile.md:26,38-39; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:820 |
| FND-002 | low | AC-5's mixed-width case names only `ill_typed`, where every other refusal in the FR names a code and a cause, and gives no return type. The checker returns `ill_typed`/`type-mismatch` at the operator for either return type. Pin `ill_typed`/`type-mismatch` and the return type, so the test asserts as exactly as the operator cases. | FR-397-AC-5 |

## Verdict

Changes requested: one medium and one low finding. AC-2's unbuildable `one`
example is SR-13720 FND-002.

Examined and clean:

- TC-1091 binds every AC: steps 1-6 and Expected Results 1-6 map one-to-one
  onto FR-397-AC-1 to FR-397-AC-6.
- Ticket acceptance 1 (Float64 add compiles and strict IR admits it) is met by
  AC-1 together with FR-093-AC-20.
- Ticket acceptance 3 (comparison settled) is met for operators by AC-5. The
  reserved intrinsic calls are still open (SR-13720 FND-003).

---
id: SR-326
title: "Composed type admission delivery gaps against FR-040 and TC-119"
type: SpecReview
analysis: gap-analysis
scope: "FR-040; TC-119; TM-003 (spec/model-linking/tests.md); src/checking/composed/; tests/composed_types.rs; tests/composed_type_pipeline.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-040
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TM-003
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-119
    type: references
---

## Summary

Gap analysis of the composed type-admission slice at 621205d. The engine
reports 350/359 matrix rows backed with **zero status lies**, and the spec
correctly keeps every FR-040 row 🚧 Planned. Type admission genuinely
implements the profile/type/query/provenance portions of TC-119 — nine of ten
ACs now carry tagged tests where none did before — but TC-119 is not
discharged: FR-040-AC-10 is `backed: false` in the engine's own minted-target
report, and the definedness half of AC-4/AC-6/AC-8 has no implementation at
all, by design.

## Verdict

**FAIL** — the full-ticket gate. FR-040-AC-10 is a matrix criterion with no
backing tagged test (FND-001) and FND-002 is high. This is the expected result
for an intentionally partial slice of compiler #40, not a regression.

**Delivered stage: CONDITIONAL.** The type-admission scope this PR claims —
source correspondence, exact nominal/scalar/unit identity, selected profile
permissions over all authored syntax including unused and unreachable forms,
forward predicate signatures with callee-owned profiles, query shape and type
obligations, capture/pre provenance, exact IR rational source normalization,
typed local/dependency/prerequisite causes, separate `Refused`/`Unfinished`/
`Typed`, bounded model summaries and reverse dependency work — is implemented
and exercised by 18 passing tagged controls. It does not discharge definedness,
evaluate family bodies, admit runtime input, emit executable IR or deliver #40,
and the artifacts say so. FR-040/TC-119, #35, #36, #39 and #40 remain open.

## Findings

| ID      | Severity | Summary                                                                                                   | Refs                                              |
| ------- | -------- | ----------------------------------------------------------------------------------------------------------- | --------------------------------------------------- |
| FND-001 | high     | FR-040-AC-10 has no backing tagged test; the engine reports it `backed: false`                              | spec/functional/FR-040-check-composed-values.md:148 |
| FND-002 | high     | AC-4/AC-6/AC-8's definedness half is unimplemented; tag-backed is not verified for those criteria           | spec/functional/FR-040-check-composed-values.md:142 |
| FND-003 | medium   | AC-9 requires zero/exact/one-short in every dimension; only Normalization gets all three, the rest only zero | tests/composed_types.rs:677                         |
| FND-004 | medium   | AC-5's N=10,001 refusal is untested although both refusal paths exist                                       | tests/composed_types.rs:469                         |
| FND-005 | medium   | Reverse gap: an admitted reference cycle yields `Unfinished` indistinguishable from exhaustion, unspecified  | src/checking/composed/solver/dependencies.rs:67     |
| FND-006 | low      | `size` via the shared `Builtin::Size` form leaves its result type unconstrained until validation             | src/checking/composed/solver/expressions.rs:132     |

## Detail

**FND-001.** `quire coverage --json` lists FR-040-AC-1..AC-9 as
`backed: true` and FR-040-AC-10 as `backed: false`. No `#[trace]` in
`tests/composed_types.rs` or `tests/composed_type_pipeline.rs` carries
`FR-040-AC-10`. The criterion — historical checker/profile/package fixtures
retain their identities, and composed typed/partial reports cannot enter
historical checked/execution interfaces by retagging or introduce an implicit
wire contract — is structurally protected: `DeclarationTypes` and `TypeReport`
have private construction (`src/checking/composed.rs:203`, `:264`) and neither
derives `Serialize`. Structural protection is not a control. One test asserting
that the historical `checking::check` and `package` paths are unchanged by the
presence of a composed `TypeReport`, and that no serialization surface exists,
would close this.

**FND-002.** AC-4 requires proved nonzero divisors and rational
normalization-before-bounds; AC-6 requires guard-before-use to *discharge*
supported presence and range obligations; AC-8 requires actual supported IR
discharge through exact formal correspondence. `admit_types` records
`ObligationKind::{Presence,ArithmeticRange,Nonzero,SumProjection,SumPrefixes,
PredicateTotal,GraphClosure,InvocationContext,CaptureInput}` and discharges
none of them — correctly, and the README and TC-119 both say so. The tests
tagged AC-4/AC-6/AC-8 verify normalization inputs, `pre` eligibility and
formal-source correspondence, which are the typing preconditions, not the
discharge. The matrix keeping these rows Planned is the accurate signal; the
per-AC `backed: true` from the engine is row-level reconciliation and must not
be read as verification.

**FND-003.** AC-9 requires distinguishing zero, exact and one-step-insufficient
limits "in every public dimension". `exhaustion_preserves_earlier_types_...`
supplies 0/127/128 for `Normalization` only
(`tests/composed_types.rs:615-676`); `Bytes`, `Declarations`, `Expressions`,
`Constraints`, `Edges`, `Records` and `Depth` get only the zero point
(`:677-717`). `public_accounting_clamps_limits_...` (`:724`) covers clamping and
atomic overflow per dimension but not the exact/one-short boundary. The
charge-before-work code is uniform so the risk is low, but an off-by-one in any
one dimension's reservation would pass today.

**FND-004.** AC-5 states N=0 and N=10,000 are checked against authored maxima
and N=10,001 refuses. The vector is `[1, 6, 10_000]`
(`tests/composed_types.rs:469`); the zero case is covered by the producer's own
`UnboundedCollection` refusal (`:462-467`), which is the right evidence. Nothing
exercises 10,001. Two distinct refusal paths exist and neither runs:
`validation.rs:180` (`wrappers` → `InvalidAggregateDomain` on a sequence type
whose maximum exceeds 10,000) and `models.rs:98` (`type_bounds` → declaration-
level `CauseKind::ModelDomain`). Which of the two a caller sees for an
over-wide model is therefore unpinned.

**FND-005.** `dependencies::finish` only enqueues declarations whose
disposition has settled, and a declaration's `remaining` counter can reach zero
only if every reference resolves to a `Typed` target. Two declarations that
call each other therefore stay `Unfinished` forever, with `exhaustion() ==
None`. `TypeDisposition::Unfinished` is documented as "required input or charged
type work did not complete" (`src/checking/composed.rs:26`), so a caller cannot
tell a cycle from a budget stop. `admit_namespace` refuses cycles upstream, so
this is not reachable today — but no FR-040 sentence allocates cycle behaviour
to this stage and no test pins it, which is the reverse (code→spec) gap this
step looks for.

**FND-006.** The shared `ExprKind::Call { builtin: Builtin::Size }` form adds no
constraint in `expression()` (`expressions.rs:132`), unlike the composed
`ValueKind::Size` form which binds its result from the qualified domain name
(`:255-260`). `validate()` does reach both through `count_domain`
(`validation.rs:98,128`), so an ill-typed use still refuses — but via
`AmbiguousType` when no context fixes the result type, rather than a cause
naming the actual problem. Consistent with "exact nominal types", worth a
sentence in TC-119 so the distinction is deliberate rather than incidental.

## Coverage

- **Plan completion:** not applicable — no plan bundle covers FR-040 and the
  owner's bookkeeping directive forbids inventing one. Recorded, not scored.
- **Matrix verification:** `quire coverage --scope <worktree> --json` at
  621205d — **350/359 backed**, **0 status lies**, 243 criteria. Six unbacked
  rows, all inherited and outside this scope: TC-115 (FR-036-AC-5/6/8), TC-010
  (NFR-005-M-1, Manual), FR-017-AC-2 (Inspection), and the three FR-036 rows
  TC-115 owns. Three unmatched tags (IT-004, `tests/fixture_audit.rs`) and 20
  untracked symbols (NFR-007 package encoding/limits) are also inherited.
  TC-119 moved from wholly unbacked to backed on nine of ten criteria.
- **Underspecified code:** no stub, no `todo!`/`unimplemented!`/`TODO`/`dbg!`
  in the new tree; no re-export-only module; no public item without a doc
  comment; no test without assertions or with a tautological one. Every new
  public item has an owning FR-040 sentence except the two reverse gaps above.
- **Semantic review (step 4):** **skipped** — the optional semantic-review
  extension is declined by standing owner direction. Intent↔test↔code agreement
  was not independently judged beyond the mechanical checks above.
- **Gates:** all local gates green at this HEAD; counts in SR-325.

## Acceptance classification

Tag-backed only (a tagged control exists; the criterion is not discharged):
AC-4, AC-6, AC-8. Fully verified for the delivered type-admission scope:
AC-1, AC-2, AC-3, AC-7, and AC-5/AC-9 minus FND-003/FND-004. No coverage:
AC-10. Full FR-040 and TC-119 acceptance, and #35/#36/#39/#40, remain open.
Producer-owned source/model canonical correspondence and relationship authority
stay real upstream gaps and are not narrowed into accepted semantics here.

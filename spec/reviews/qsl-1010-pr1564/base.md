---
id: SR-17227
title: "spec-review of the hyper query request, FR-315 amendment (quire-spec-language PR 1564)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; git diff origin/main...<sha removed> (PR 1564, branch qsl-1010-fr315-seam-2): spec/functional/FR-315-encode-the-smt-lib-transition-relation.md, spec/functional/FR-487-lower-hyper-atoms-and-match-conjuncts-to-checked-nodes.md, spec/test-cases/TC-1120-the-hyper-self-composition-smt-queries-are-canonical.md, spec/test-cases/TC-1190-the-hp-2-product-identity-binds-each-smt-outcome-to-its-item.md, spec/test-cases/TC-1264-hyper-atoms-and-match-conjuncts-are-lowered-and-evaluated.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-315
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-487
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1120
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1190
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1264
    type: references
---

# Spec review of QSL-1010, the hyper query request (quire-spec-language PR 1564)

## Summary

Ticket: QSL-1010. One folded pass (spec-review with the EARS, integrity, evidence and failure-domain checks) over `git diff origin/main...<sha removed>. The branch merges cleanly with main (<sha removed>, whose only spec change is a wording fix in FR-488). `quire validate` reports no error for the five files. Read against merged FR-487, FR-488, FR-176, FR-182, FR-336, FR-315-AC-7 to AC-13, AC-43 and AC-45, and against the code on main.

Code on main, checked symbol by symbol:

- `CheckedStepMatch::new(&CheckedPackage, &CheckedHyperClause)` exists (`qsl-eval/src/hyper/checked.rs`). `universal(&self, &[ParameterBinding], &[StepLabel], &mut Meter) -> Result<MatchReading, MatchIncomplete>` evaluates conjuncts in index order, each once, and stops at the first that is not `Completed(true)`, as the spec states. The instance argument is ignored, so the spec's "readings do not depend on them" is true.
- `MatchReading` has `Holds`, `Fails`, `Undefined{conjunct,cause}`, `Refused{conjunct,outcome}` and already has `Fault(InternalFault)` in code; the FR-487 edit only brings the spec to the code. `CheckedStepMatch::read` returns `Fault` for `CallFailure::Fault` and for any other call failure.
- `ParameterBinding{parameter, instance: ModelInstance{population, key}}` and `StepLabel::{Operation(&ModelTransition), Stutter}` exist. `ObjectParameter.population` is public, so the encoder can build the bindings.
- `CheckedHyperClause::{condition, component_reads, atom_slots, mu_universal, occurrence, assembled_occurrence}` exist with the signatures the spec uses. `assembled_occurrence()` is `Option<&OccurrenceKey>` and `occurrence()` is the zero key for an unassembled clause, as stated.
- `SubjectIdentity::of(&ModelSubject) -> Result<_, SubjectIdentityRefusal>` refuses when the package has no emission. `encode_smt_query(subject, item, kind, limits)` exists. `SmtEncodingRefusal::Fault(InternalFault)` exists. `Reader::boolean(CheckedNode) -> Result<(Term, Term), _>` exists.
- `MonitorTable::of(clause: &CheckedTemporalClause, max_atoms, max_states)` calls `translate(clause, max_states)`. `translate_body(body, profile, max_states)` calls the same `translate_formula` with `Activation::Origin`, `Selection::Clause`, `Counting::Every` and no symmetry as `translate` does for an `on origin` clause. A constructor over `(CheckedFormula, profile)` that `of` delegates to for `Origin` is therefore byte-preserving, and the implementation order (constructor refactor first, then the hyper encoder) is sound: the refactor is invisible to AC-43 and AC-45 and gives the encoder its tabulation.
- `HyperQueryRequest`, `HyperQuery` and `encode_hyper_query` do not exist in code. That is the ticket's work, not a spec defect. `Reader` has no slot binding or copy-indexed `component_reads` reading yet; that is implementation scope.

Internal consistency: the five-member request, the symbolic atom reading and the evaluated `μ_U` are stated once and used the same way in the Joint steps, Property, Refusals, Faults and AC text. The refusal order (identity four, then `hyper-body`, `query-kind`, `constants`, per-copy sorts and domains, `match-conjunct`, `applications` and monitor limits, per-copy `out-of-fragment`, `script-bytes`) puts `match-conjunct` before `out-of-fragment`, which is what AC-48's second sentence needs. AC-46's nine tuples are AC-7's nine; AC-47's tuple `(step(0), step(0))` is the first tuple on which the two original conjuncts hold and is the tuple AC-10 and FR-487-AC-8 name. "Nine universal calls" is correct for every kind, depth and instance because the tuple set is the product of three component disjuncts per variable and one evaluation serves every step. AC-47's `ieee_not_exact` conjunct is FR-487-AC-5's, and AC-48's body is FR-176-AC-5's, whose non-literal divisors are `out-of-fragment` by FR-315 table row 15. FR-487's `Fault` text, FR-315's Faults bullet and the Internal faults paragraph agree. AC-49 agrees with FR-488's note that an unassembled clause has no occurrence.

Earlier findings (SR-17048 to SR-17053, PR 1484), each re-checked:

| Earlier | Status | Where |
| --- | --- | --- |
| SR-17048 FND-001 (letters cannot read atoms at a symbolic state) | resolved | atoms are read symbolically from `condition`/`component_reads` through `Reader::boolean`; no `HyperLetters` |
| SR-17048 FND-002 (`match-conjunct` needs the conjunct) | resolved | `Refused{conjunct,..}` and `MatchIncomplete{conjunct,..}` carry it; refusal names index, site and tuple; AC-47 |
| SR-17049 FND-001 (TC-1190 seam results) | moot | seams removed from AC-12 and TC-1190 |
| SR-17049 FND-002 (`instance` vs `ParameterBinding` types) | resolved | the encoder builds one binding per parameter |
| SR-17049 FND-003 (layout) | not re-raised | the new lines are wrapped |
| SR-17050 FND-001 (seams that answer equally) | moot | "The encoder owns its readers" |
| SR-17051 FND-001 (no AC for a `Refused` matcher) | resolved | AC-47 and TC-1120 step 6 |
| SR-17052 FND-001 (seams unreachable through `qsl-replay`) | moot | the facade surface is the request only |
| SR-17053 FND-001 (misbehaving seam) | resolved | the Faults bullet |

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-315 | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md | examined |
| FR-487 | spec/functional/FR-487-lower-hyper-atoms-and-match-conjuncts-to-checked-nodes.md | examined |
| TC-1120 | spec/test-cases/TC-1120-the-hyper-self-composition-smt-queries-are-canonical.md | examined |
| TC-1190 | spec/test-cases/TC-1190-the-hp-2-product-identity-binds-each-smt-outcome-to-its-item.md | examined |
| TC-1264 | spec/test-cases/TC-1264-hyper-atoms-and-match-conjuncts-are-lowered-and-evaluated.md | examined |
| FR-488 | spec/functional/FR-488-emit-a-checked-hyper-clause-as-a-v2-hyperproperty-node.md | context_only |

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | FR-315-AC-46 and TC-1120 step 5 cannot be run as written for `encode_hyper_query`. The AC says the call "leaves the request's meter with the charges that nine `CheckedStepMatch::universal` calls make" and step 5 reads "each meter's charge log". But `encode_hyper_query(request: HyperQueryRequest<'_>, kind)` takes the request, and its `meter: Meter`, by value, and `Meter` is a plain `Clone` value (`quire-exact`), so a clone does not share counters with the one the encoder spends. The caller has no meter to read after the call. Only `HyperProductIdentity::of(&request)` leaves the meter readable, and that half of the AC is about a meter that is never charged. Fix: state how the charge is observed, for example by budget (a request whose budget equals the nine calls' total encodes, and one unit less refuses `match-conjunct`, for each kind, depth and instance), or by `HyperQuery`/the refusal returning the spent meter, or by taking the meter as `&mut Meter`. Also say whether the one meter is a single cumulative budget over all tuples, which the text implies ("the budget for evaluating `μ_U`") and step 5 treats as nine independent fresh meters. | FR-315 "Hyper self-composition queries" `HyperQueryRequest`; FR-315-AC-46; TC-1120 step 5 | correct-requirement-no-evidence |
| FND-002 | low | The Faults bullet is not placed in the refusal order. The order paragraph lists the refusals and not `Fault`, yet a `MatchReading::Fault` arises at the `match-conjunct` position and a missing atom condition at the atom-reading position, so a request that both refuses (for example `too-large` constants) and would fault gives an unspecified result. Say that faults are raised at the point of the broken read, in the same order, and that a refusal listed earlier wins. | FR-315 "Product identity" refusal order; Faults bullet | missing-requirement |
| FND-003 | low | The `constants` `too-large` check is placed before the per-copy `out-of-fragment` refusal, but an atom outside the fragment prints no term, so its constants cannot be counted. State that the `constants` measure counts the terms the encoding printed up to that point, or move the per-atom `out-of-fragment` before `too-large constants`. AC-48's two cases do not exercise the order. | FR-315 refusal order; FR-315-AC-48 | missing-requirement |
| FND-004 | low | FR-315-AC-48's last sentence ("Every script FR-315-AC-7 to FR-315-AC-9 and FR-315-AC-43 give is read by a `QF_LIA` solver without a nonlinear construct") is verified by TC-1120 step 8, which submits only the AC-7 base query, the AC-8 depth-1 step query and the `self.l = self.h` base query. AC-9's and AC-43's scripts are not submitted, and step 8 is skippable. Name the scripts step 8 submits, or tag the AC-43 query in TC-1264 step 15; keep the "no nonlinear construct" claim as a syntactic check on the script text, which does not need a solver. | FR-315-AC-48; TC-1120 step 8 | correct-requirement-no-evidence |
| FND-005 | low | FR-315-AC-50 and TC-1264 step 15 say the scripts of AC-43 and AC-45 are "byte-equal before and after `MonitorTable::of` delegates to it". A test cannot compare with code that no longer exists. State that the expected bytes are golden files recorded from main before the refactor and committed with it, which also makes the constructor-first order checkable. | FR-315-AC-50; TC-1264 step 15 | correct-requirement-no-evidence |
| FND-006 | low | The Faults bullet adds six SHALL behaviours and no AC covers any of them, and `MatchReading::Fault` has none in FR-487. Accepted: the Internal faults paragraph already withholds ACs from invariants "a well-formed request cannot break", and the encoder owns `CheckedStepMatch`, so no public input reaches a `Fault`. Record that reasoning in the Faults bullet (one sentence) so the omission reads as a decision, and keep the code tested by unit tests at the encoder boundary. | FR-315 Faults bullet; FR-487 `Fault` bullet | wrong-requirement |
| FND-007 | low | FR-487's `Fault` bullet says "FR-315's `encode_hyper_query` returns `SmtEncodingRefusal::Fault` for it (FR-315 'Refusals')". The rule is in the Faults bullet, with the paragraph named "Internal faults". Point to those. | FR-487 `MatchReading` bullets | wrong-requirement |

## Verdict

PASS with required fixes: no high finding, one medium. FND-001 must be fixed in this PR, because AC-46 as written has no way to fail on a charge for the call it names. FND-002 to FND-007 are low and go in the same round. FND-006 is the answer to "is the missing Fault AC acceptable": yes. The claimed implementation order, constructor refactor first with golden bytes, is sound (FND-005 makes it testable).

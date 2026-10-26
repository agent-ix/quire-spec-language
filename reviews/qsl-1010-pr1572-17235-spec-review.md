---
id: SR-17235
title: "Spec review of the FR-315 tuple-naming sentence and the hyper-query ACs delivered by QSL-1010 PR B(b)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@c8ea7587198f5b68eb1c209078a713d40bdbe925; PR #1572 (branch qsl-1010-encode-hyper-query), QSL-1010 PR B(b) and QSL-1083: spec/functional/FR-315-encode-the-smt-lib-transition-relation.md (the amended `match-conjunct` bullet; context Hyper self-composition queries, AC-7 to AC-14, AC-43 to AC-52), FR-487, FR-182"
review_set: subset
date: '2026-10-25'
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-315
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-487
    type: references
---

## Summary

Ticket: QSL-1010 (QSL-1083). Folded pass (EARS, integrity, failure-domain) on the one sentence this PR adds to FR-315 and on the ACs it implements. The added `match-conjunct` sentence (each tuple component as the operation's declared name applied to its arguments in decimal, `step(0)`, led by the receiver's key and a dot, `v.step(0)`, when the subject holds more than one object, or `stutter`) is coherent with AC-10 and AC-47, whose tuples `(step(0), step(0))` come from a one-vault universe, and with the code (`Application::label`: receiver key only when `subject.objects.len() > 1`). It is silent for non-integer arguments (FND-002), and no AC exercises the receiver-key form. The `always`-over-a-state-predicate clause, read as any propositional combination of `holds` atoms, is sound and equal to the monitor path (SR-17233, 96-case cross-check), but the spec never says so (FND-003). The tuple-order, undefined-match, distinct-state, per-copy read, fairness and meter clauses are implementable and were found consistent with each other.

## Verdict

**CONDITIONAL** — FND-001 and FND-004 need a spec edit in this PR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-315-AC-14 names `HyperSmtOutcome` and `settle_hyper_smt`, which do not exist (FR-182, FR-182-AC-12); the AC cannot pass until FR-182 lands, yet it sits in the requirement this PR completes and the "Product identity" prose says CG "calls ... `settle_hyper_smt`". Split the AC: the encoder-and-identity half (types, `encode_hyper_query`, `HyperProductIdentity::of`, no struct literal, no serde) belongs here; the settle half belongs to FR-182. The compile-only consumer then verifies the whole of the new AC-14. | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:1242, 1121-1131 |
| FND-002 | low | The tuple-naming sentence defines arguments only "in decimal". The encoder prints a Boolean as `true`/`false`, an enum as `#<rank>`, any other value as its canonical text and an unprintable one as `?`. A refusal text is evidence for a human and for CG's report; say what a non-integer argument looks like, or restrict the sentence to Integer arguments and say others are not specified. No AC exercises the receiver-key form `v.step(0)`; one over a two-object universe would pin it. | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:1064-1069; qsl-eval/src/simulation/model/symbolic.rs:112-132 |
| FND-003 | low | "A body that is `always` over a state predicate" is not defined. ADR-018's TP-1 is a single atom; the encoder takes any combination of constants, `holds` atoms, `not`, `and`, `or`, `implies` under an unbounded `always`. That is sound (same violations and definedness failures as the monitor, checked by the cross-check), but a reader of FR-315 cannot tell whether `always (holds(A) implies holds(B))` goes through the monitor (AC-43 only says `always (A implies always B)` does). State the fast-path shape, and that the two paths give equal satisfiability. | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:1010-1015; qsl-eval/src/smt/hyper.rs:428-461 |
| FND-004 | medium | "Refusals and their order" places `match-conjunct` before `too-large` for `applications`, and defines the latter as the joint disjuncts of one step, but also requires each component's disjuncts to be those of the single-subject step relation, whose own count refusal is also `applications`. The text does not say which fires first when a component alone exceeds the limit, so the implementation chose the component count first (SR-17233 FND-001). Decide it in the spec: either the component count is checked at the `applications` position (after `match-conjunct`), or it is checked first and `match-conjunct` cannot precede it. | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:1058-1062, 1186-1192 |
| FND-005 | low | AC-12 uses a vault with the universe {v, w}; AC-46 says "the same request at instance `w`" leaves the same counters and counts nine tuples. With two objects each component has four operation disjuncts and the stutter (25 tuples, `v.step(0)`-style labels), not nine. The AC-46 test resolves this with a universe {w} for the w case. Also note, for CG's reading of V-3: over the secure vault with universe {v, w}, `match { a.step.op = b.step.op and inputs equal }` does not compare receivers, so `(v.step(1), w.step(1))` is a matched tuple that makes `v.l` differ, and instance `v`'s base query is satisfiable (the encoder is right; the clause is not inductive there). State the universe in AC-46. | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:1240, 1275 |
| FND-006 | low | AC-50 requires FR-315-AC-43's script to be byte-equal to golden bytes "recorded from `main` before `MonitorTable::of` delegates to the constructor". No hyper encoder exists on main before this PR, so the hyper goldens (secure-base-1.smt2, secure-step-1.smt2) can only be recorded from the new encoder, and are a regression check, not an equivalence check against the pre-refactor path. The goldens are correct (verified against a hand-derived table: states 0 initial, 1 rejecting, 2 pending; moves as AC-43 describes; unchanged between the first commit and the head), so amend AC-50 to say the AC-45 TP-3 goldens carry the equivalence and the hyper goldens the encoder. | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:1279; qsl-replay/tests/golden/hyper_monitor |

## Coverage

Examined: the amended `match-conjunct` bullet against AC-10, AC-47, AC-12, AC-46 and `Application::label`; the property, negated-property, undefined-match, joint-step, copies, initial-state, fairness and meter clauses against `hyper.rs`; the Faults paragraph against the code's arms; FR-487 as the encoder consumes it (`CheckedStepMatch::universal`, `component_reads`, `atom_slots`). Not assessed: ADR-023 HM-4/HC-1 beyond what FR-315 quotes.

## Dispositions

(none yet)

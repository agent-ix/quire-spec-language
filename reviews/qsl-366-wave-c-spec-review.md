---
id: SR-1120
title: "QSL-366 Wave C spec review of ADR-018 and FR-123 to FR-128 (#562)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@e6a5fb5690e54e057e39cfd2d1a72744355d6473; git diff origin/main...HEAD over spec/: ADR-011, ADR-013, ADR-014, ADR-016 amendments; ADR-018; FR-072, FR-101; FR-123 to FR-128; US-015; TC-453 to TC-455, TC-518 to TC-523, TC-536 to TC-539; spec/spec.md; spec/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-124
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-125
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-128
    type: reviews
---

## Summary

Ticket: QSL-366. PR quire-spec-language#562, the base of a stack of 10.
One review file for the PR, covering spec-review base, EARS, integrity and
scope-boundary checks over the PR diff only.

Checked against the eight owner rulings recorded on QSL-366: explicit-state
first (RU-1); unmarked fairness is weak whole (FA-6); deadlocks reported by
default with the `terminal any` opt-out (DL-1 to DL-7); mixed formulas
admitted (IV-1 to IV-7); Büchi cost and the interval limit stated (IV-5,
IV-6); undefined evaluation refutes with `UndefinedEvaluation{where, cause}`
(UE-1 to UE-6); default limits 1e7 states and 1e8 transitions (FR-101,
FR-126); `max_depth` is the search horizon `k` (§1 "Depth is a method
parameter", FR-126). ADR-018 and FR-123 to FR-128 follow every ruling.

`quire validate` on every changed spec file passes (one pre-existing
`ac:vague-response` warning on FR-101-AC-14); the index check passes.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-101 still makes depth a limit kind. The PR's new Inputs text calls `max_depth` the search horizon, but the same paragraph says "A run that reaches a limit names it and its value in `Outcome::Bounded`", and FR-101-AC-7 still has a run that reaches `max_depth` return `Outcome::Bounded` "with that `Limit`", category incomplete. ADR-018 §3 keeps `explore::Outcome::category()` on that map. This contradicts the QSL-366 ruling that `max_depth` is the horizon `k`, not a limit kind. | spec/functional/FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md:63-71,333; spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:246-247 |
| FND-002 | medium | The base PR cites artifacts that exist only in later PRs of the stack. FR-126 adds `CheckSubject::Protocol`, `max_live_instances`, `max_store_buffer` and `max_messages`, citing FR-205, FR-209, FR-215 and FR-225, and FR-126-AC-8 and TC-536 test those members. ADR-018 adds TP-5, `MappingUndetermined` (V-6), `RefinementFailure` (CX-2) and the IV-7 transfer rule, citing ADR-020 RE-1 to RE-4, RC-1, CO-3 and CO-4. None of FR-205, FR-209, FR-215, FR-225 or ADR-020 exists in this tree or on main. Each amendment belongs in the PR that adds the record it cites. | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:63,74-76,93-97,299,319; spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:126,151,161,281,476 |
| FND-003 | medium | The FR-101 sampler change belongs to another lane. It adds `choice` to the sampler preimage and replaces the FR-101-AC-3 vectors (step-0 digest `d5160380…`). It cites "QSpec TC-210's vector", but QSpec main's TC-210 still carries `cb7d4b3b…` with no `choice` member. The new vector exists only on QSpec branch `spec/wave-b-q4-prob-time` (probabilistic, STD-137). This is not QSL-366 temporal work, and it moves TC-454 from implemented to 🚧. Dropping the `1-draft.1` revision pin is the QSL-395 ruling and is correct; the `choice` member is not. | spec/functional/FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md:246-268,329; spec/test-cases/TC-454-the-pinned-sampler-reproduces-its-vectors-and-sampled-traces-replay.md |
| FND-004 | medium | FR-124 classifies a terminal state as intended when `P` evaluates `true` and "as **deadlocked** otherwise". So a `terminal when P` that evaluates undefined classifies the state as deadlocked. ADR-018 UE-1 and FR-126-AC-9 settle that case as a counterexample of kind `UndefinedEvaluation{where: 3, cause: division-by-zero}`, not `Deadlock`. | spec/functional/FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md:89-91 |
| FND-005 | medium | FR-125 defines a claim's letter as "the value of every atom of the claim at that position". It omits UE-1's third part, DL-1's `P` at a terminal state for the deadlock-freedom item. FR-128 replays an `UndefinedEvaluation` counterexample by evaluating "the letters of the replayed positions … by FR-125", and its `Deadlock` branch handles only `P` false or absent. So the FR-126-AC-9 deadlock-freedom counterexample (`terminal when 6 / (3 - c.value) = 0`) has no replay rule that reproduces it, and no FR-128 AC covers it. | spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md:138-140; spec/functional/FR-128-replay-a-model-counterexample.md:140-155 |
| FND-006 | low | FR-126 pairs each initial model state with "the automaton's successor from its initial state on position 0", in the singular. The TP-4 automaton is a nondeterministic generalized Büchi automaton with a set of successors, and ADR-018 §6 and FR-126-AC-1 need `(0, 0, q1)` as an initial product state beside `(0, 0, q0)`. | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:174-175 |
| FND-007 | low | The rewritten FR-101 sampler paragraph keeps "STD-109 settles it upstream" in requirement prose. Ticket ids belong only in References. TC-454's title and index row still say "pinned sampler" after the pin was removed. | spec/functional/FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md:266; spec/tests.md:233 |

## Verdict

Not mergeable as it stands. ADR-018 and FR-123 to FR-128 are consistent
with every recorded ruling. EARS phrasing holds; every AC has a behaviour TC
with concrete values; there are no new pins, ledgers, caps or compat paths;
and QSpec semantics are cited (FR-125's restatement is marked informative).

The worked examples check out by hand:
- §6's 15-state product and both fairness verdicts;
- the SM-8 lasso example;
- FR-126-AC-5's `max_depth` 2/3 boundary;
- FR-130's inputs to the stack.

To fix:
- FND-001: FR-101 still treats depth as a limit kind, against a named ruling.
- FND-002: forward references to records in later PRs of the stack.
- FND-003: an off-lane sampler change.
- FND-004 and FND-005: leave an undefined `terminal when` predicate without one consistent classification and replay path.

## Dispositions

Round 1, reviewed at 18a9c22bd1783e2d40eaf3528e662f943183e2b8.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 18a9c22b |
| FND-002 | fixed | 18a9c22b |
| FND-003 | accepted-no-change | The team leader kept the `choice` member to align FR-101 with the FR-181 preimage of QSpec #175 (agent-ix/quire-specification, open draft, STD-137/139). The vector is real on #175 and I recomputed the step-0 digest `d5160380…`. FR-101-AC-3 now says it is TC-210 as amended by QSpec FR-181. It agrees with QSpec main only if #175 merges before #562. |
| FND-004 | fixed | 18a9c22b |
| FND-005 | fixed | 18a9c22b |
| FND-006 | fixed | 18a9c22b |
| FND-007 | fixed | 18a9c22b |

Round 2, reviewed at ed7bcc8b1226619455e5e8e2a25c9815deb35c03. No finding was open after round 1, so this round adds no row. It adds FND-008 to FND-020 below, found in the commits after 18a9c22b (the certificate checkers FR-314, FR-338 and FR-339, FR-337, the limit and cancel settlement, and `[a,*]` under infinite-trace).

## New findings (disposition pass 2)

Reviewed at ed7bcc8b1226619455e5e8e2a25c9815deb35c03 (`git diff 0ecfb9e9...ed7bcc8b`, plus the cross-PR checks of the brief). Checked against QSpec main aa031866.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | high | FR-338 rejects with `BadState` any reached state that "is deadlocked (FR-124)", for every item, and FR-339 inherits it for TP-4 (it drops only the monitor part). ADR-018 DL-6 says a deadlock changes no authored claim's verdict, and FR-126 ends only a `DeadlockFreedom` item's first phase at a deadlock. Over the `Counter` subject with no `terminal` member, `always holds(c.value <= 3)` returns `Holds{Exhaustive}` with a closure certificate, and `check_closure` rejects it `BadState` at value 3, so the item settles `inconclusive`, `CertificateRejected`, instead of `proved`. The deadlock-freedom item's own monitor already rejects at a deadlocked state. Delete the deadlock clause from `BadState` and from PC-3 (c). | spec/functional/FR-338-check-an-en-1-closure-certificate.md:83-86; spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:178; spec/functional/FR-339-check-an-en-1-component-certificate.md:72-73 |
| FND-009 | high | An `Inductive{depth: k}` proof needs two refutations: the base case and the step case. FR-314 derives "the base case and the step case at `k`" but `SmtProofCertificate` carries one `query` and one `proof`, and the checker accepts when that one proof concludes the empty clause. If the derived query holds both cases in one script, refuting it proves only that one of them is unsat, so a certificate that refutes only the step case is accepted and the item settles `proved` with no label. Carry one query and proof per case and require both, or state how the two cases are encoded so one refutation covers each. No AC covers `Inductive`. | spec/functional/FR-314-check-an-smt-proof-certificate.md:42-49,71-75; spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:181 |
| FND-010 | medium | FR-314 accepts any step that "is an instance of an Alethe rule whose premises are earlier steps or assertions". Alethe has rules with no checkable semantics, `hole` and `lia_generic` among them, which a solver emits where it does not justify a step. Under that wording a proof with a hole passes and the item settles `proved` with no label. Name the rules the checker admits and reject a step of any other rule. | spec/functional/FR-314-check-an-smt-proof-certificate.md:79-81 |
| FND-011 | medium | `QueryMismatch` compares the certificate's query with QSL's derived query "in SMT-LIB canonical printing". SMT-LIB defines no canonical printing, and equality also needs CG's SMT backend to emit the same declarations, names and assertion order that `qsl-eval`'s encoding derives. Neither a shared encoder nor a canonical form is specified, LA-2 lists no SMT-LIB encoding in `qsl-eval`, and RU-1 says QSL has no SMT code. Two implementations would disagree on every certificate. State the canonical form, or have the backend use QSL's encoder. | spec/functional/FR-314-check-an-smt-proof-certificate.md:71-78; spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:181,428 |
| FND-012 | medium | FR-314-AC-1 and AC-2 prove the TP-1 claim `always holds(c.value <= 3)` `BoundedComplete{depth: 4}`. ADR-018 V-2 gives `BoundedComplete` only to TP-2 `on origin` at `k >= h`, and TP-1 gets V-3 `Inductive` or V-5. The vector uses a basis the SMT backend never returns for that form. Use a TP-2 `on origin` clause for `BoundedComplete`, and add an `Inductive` vector (FND-009). | spec/functional/FR-314-check-an-smt-proof-certificate.md:91-92; spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:147 |
| FND-013 | medium | No requirement says who calls `check_smt_proof`. FR-314 names FR-127 as "the settlement that runs the check". FR-127 says its map "covers QSL's native engine, EN-1, only" and that CG's SMT map constructs the SMT values, and its table has no SMT row. ADR-018 LA-3, PC-5 and DS-1 list the checkers as PC-3 and PC-4 only. State the caller (FR-127's map for an SMT `Proved`, or the driver before CG's map) and add FR-314 to LA-3 and PC-5. | spec/functional/FR-314-check-an-smt-proof-certificate.md:98-99; spec/functional/FR-127-settle-a-model-check-verdict-as-a-terminal-record.md:164-176; spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:180,429 |
| FND-014 | medium | ADR-018 CX-2 and FR-128 list the counterexample's members as `kind`, `initial_state`, `prefix`, `loop`, `over_binding`, `fairness` and `trace_position`, and CX-2 says QSL "keeps no member list of its own". QSpec FR-364 at aa031866 also has `trace` (the arm) and `interval`, and its replay step 5 refuses `stale_dependency`/`content-mismatch` naming `interval` when the payload's interval differs from the recompiled clause's. FR-128 has no interval check, so a counterexample produced for another interval of the same clause replays. Drop the member list and cite FR-364's table, and add the interval check and an AC. | spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:304; spec/functional/FR-128-replay-a-model-counterexample.md:68-74 |
| FND-015 | medium | FR-128 reproduces an `UndefinedEvaluation` counterexample when the first undefined position is `where` "with an undefined cause equal to `cause`". QSpec FR-364 step 6 evaluates the expression the envelope's `where.locus` names (and `where.transition` for a guard) and requires the locus and the reason to agree. FR-128 never compares the locus, so a payload naming the wrong expression with the same reason at the same position reproduces under FR-128 and disagrees under FR-364. Compare the locus too. | spec/functional/FR-128-replay-a-model-counterexample.md:134-152 |
| FND-016 | medium | SM-8's past reach has cases for a past interval operator with upper bound `b` and for unbounded past operators, and none for a past operator with an `[a,*]` interval, which IV-1 and IV-2 admit under infinite-trace (bb271b5a). For `once[2,*] p` the unroll count is undefined, so the exact lasso semantics SM-1 rests on has no rule. Add the case: the operand reach plus `a + L`. (FR-329 in #585 restates SM-8 and has the same gap, SR-1130 FND-017.) | spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:225 |
| FND-017 | medium | `ModelCheckLimits` embeds FR-101's `Limits`, and IV-6 says "EN-1's limits are FR-101's `Limits` plus `max_automaton_states`". But `ModelCheckLimits::default()` publishes `max_states` 1,000,000 and `max_transitions` 10,000,000, while `Limits::default()` is 10,000,000 and 100,000,000 (FR-101-AC-15, TC-536 step 1). The embedded type has two defaults. SR-1120's summary recorded the QSL-366 ruling as 1e7 states and 1e8 transitions for FR-101 and FR-126; only the commit message of 5f9d4fea attributes the change to an owner ruling. State one default, or give `ModelCheckLimits` its own members. | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:70-73,83-84; spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:510 |
| FND-018 | low | FR-127's Outputs declares the certification member twice: "`TerminalValue::Proved` carries `certification: Option<Certification>`", then `TerminalValue::Proved { basis: ProofBasis, certification: Certification }`. The ACs write `Proved{Exhaustive, Certified}` and AC-9 writes `Some(Uncertified)`. Keep the `Option` form. | spec/functional/FR-127-settle-a-model-check-verdict-as-a-terminal-record.md:86-89 |
| FND-019 | low | The ADR-013 O-16 amendment says `Certified` "when a core certificate checker accepted the proof's certificate". PC-1, FR-127 and FR-314 give an SMT proof whose certificate FR-314 accepts no label. State the SMT case. | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:417-419 |
| FND-020 | low | ADR-018:64 says a bare FR id is a QSL requirement, but UE-4, §3 and DL-4 cite a bare "FR-181" for canonical breadth-first order. That is QSpec FR-181, which QSL implements as FR-101. Write "FR-101" or "QSpec FR-181" (the same defect as SR-1125 FND-009 in ADR-019). | spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:193,244,496 |

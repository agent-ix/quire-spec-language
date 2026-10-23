---
id: SR-16197
title: "spec-review of the FR-315 SMT proof-query points (quire-spec-language PR 1289)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@7dd075ef; spec/functional/FR-315-encode-the-smt-lib-transition-relation.md; spec/test-cases/TC-900-the-smt-lib-encoding-is-canonical.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-315
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-900
    type: references
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-125
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: references
---

# Spec review of the FR-315 SMT proof-query points (quire-spec-language PR 1289)

## Summary

Ticket: STD-338. This is one spec-review pass over
`git diff origin/main...7dd075ef`, which touches FR-315 and TC-900 only. The
oracles were ADR-018 (V-2, SM-4, SM-6, UE-1, UE-4, UE-6, EN-3, FA-2, DL-1,
DL-2, DL-3, DL-7, CX-3, IV-7), FR-120 (value domains, contract clauses,
post-states, result pairs), FR-125 (Instancing, FR-125-AC-2, FR-125-AC-10),
FR-314-AC-1, TC-1084, merged #1282 (Product identity) and merged #1288 (HP-2
solver-stop precedence). The code-review and gap-analysis methods did not
run because the diff has no code.

Gates: `make check-spec-validation` (QSPEC_DIR=~/dev/quire-specification)
exits 0 with no warning on FR-315 or TC-900. origin/main moved to 2c40f78b8
(#1288) after this head was cut. `git merge-tree` reports a clean merge, so
main is not yet an ancestor of the head.

Examined, clean:

- **The stutter keeps unsat sound.** The proof-query stutter adds edges and
  no state, so the exact relation (operations plus SM-4's terminal stutter)
  is a subset of the encoded one. Every exact path to `depth` is therefore
  an encoded path, and an unsat Unrolling or InductionBase query is unsat
  for the exact relation. In the step case, distinctness rules out a stutter
  between two copies that agree in every constant and in the monitor state.
  Without the stutter, a behaviour that ends before `depth` would drop out
  of the query. With it, every copy has a continuation. A sat model is only
  a candidate, and FR-128/CX-3 replay gates it.
- **The h-flag reading of false-extension is exact.** On a live copy that
  is not terminal, the first conjunct forces `h<i+1> = h<i>`, so an
  operation step is required. On a live terminal copy, no operation disjunct
  can hold, because the frame and range assertions make `s<i+1>` one of
  FR-120's candidates and enabledness is the exact existential over them.
  The second conjunct then forces `h<i+1>`. Once a copy is past the end, it
  stays past the end. Hand-checked against `Counter`:
  `Unrolling{3}` for `always[0,3] holds(c.value <= 3)` is unsat, which is
  FR-314-AC-1's V-2 vector. `always[0,5]` at depth 5 is sat through `h4`,
  as FR-125-AC-2 has it. The `eventually[0,3]`/`[0,2]` results match
  AC-18. Refusing `query-kind` for the induction kinds keeps V-2 as the only
  reading.
- **The shortest-path argument holds for definedness.** Take a shortest
  path to the first state that has a violation or a definedness failure.
  It is simple, and every earlier state is clean. If it has `k` steps or
  fewer, the base case catches it. Otherwise its last `k+1` states satisfy
  the step case's hypotheses. Every failure kind reads copy `p` and literals
  only, so an `InductionStep` that reads guard and postcondition failures
  at `depth` is sound. The postcondition-failure term matches FR-120's
  set-of-outcomes rule (no clause false, some undefined) and UE-6. The
  precondition-failure term matches FR-120's ascending-name short-circuit.
- **DL-7 is encoded as stated.** `deadlocked` is the negation of FA-2
  enabledness, built by the same candidate expansion, conjoined with `not P`.
  P counts as false when no `terminal when` is declared, and an undefined P
  at a terminal copy is a UE-6 failure. The Tally deadlock results in AC-16
  and AC-20 check out by hand: (2,3) is terminal, and `t.tally = 2` makes it
  intended.
- **The Tally vectors check out.** AC-4, AC-15, AC-16 and AC-19 hold over
  the Tally subject. The guarded coercions are TC-1084's admitted shape. G
  and C are never undefined on a reachable state, so the AC-16 base and
  step at depth 1 for `t.tally < 3` are unsat.
- **The monitor numbering is deterministic.** It is a breadth-first
  tabulation over letters in ascending numeric order. The monitor is
  deterministic and total over the tabulated states, so the implications
  fix `m<i>` from `m<i-1>`. The step case leaves `m0` open, which
  over-approximates and is sound.
- **Fixed size limits are consistent with FR-314.** FR-314's
  `CertificateCheckLimits` bound only the certificate check, and the
  rebuild takes no encoding setting. A `too-large` refusal is not a V-7
  stop, which agrees with #1288's rule that a limit bounds an SMT run only
  when its definition names an EN-2 or EN-3 query.
- **No conflict with #1282 or #1288.** The Product identity section is
  untouched, and the hyper component-stutter rewording keeps HC-1 and HC-4
  sound. The rewording is the same over-approximation, and an HV-8 prefix
  still settles only through FR-183's replay.
- **Ids are unique.** FR-315-AC-15 to AC-23 and TC-900 steps 7 to 15 appear
  in no other open PR. #1287 tags tests with AC-1 to AC-5 and TC-900 only.
  This PR rewrites AC-4 from `Slot` to `Tally`, so #1287's AC-4 test, which
  still uses the `Slot` fixture, must be retargeted.
- **The lead defaults are labelled.** The PR body labels its defaults as
  lead defaults. It raises the TP-2 `on each` narrowing against ADR-018's
  EN-3 row, and the open question of V-2 definedness past `depth`, as owner
  questions. Neither is repeated as a finding here.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | FR-315-AC-22's instance order contradicts FR-125. FR-125 orders instances by population declaration identity, then by the population's universe list order. FR-125-AC-10 says this explicitly: `[d, c]` gives `d` then `c`, "not an ascending reference order". AC-22's universe `{d, c}` therefore gives instances `d` then `c`, and the negated property's first two disjuncts at position 2 are `d`'s violation then `c`'s. AC-22 expects `c` then `d`. The declaration and step-disjunct orders, by object key, are correct (`c`, `d`). Only the instance order in the AC is wrong. Fix: expect `d` then `c`, or state that FR-315's instance order is FR-125's canonical order and keep the universe-list vector. | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:238-244, :657; spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md:110-115, :227 | wrong-requirement |
| FND-002 | medium | Operations that declare a result have no encoding and no refusal. FR-120 pairs each candidate with each value of the result domain, and a candidate is a successor when some pair's postcondition is true (FR-120-AC-10's `isZero(): Boolean` with `post Z { result = ... }`). FR-315's operation disjunct, its enabledness ("some candidate post-state satisfies all of its postcondition clauses") and its postcondition-failure expansion have no `result` constant and no expansion over the result domain. None of the 15 codes covers this case. A coder must guess. Treating `result` as unconstrained, or refusing the item, gives different verdicts. Fix: expand enabledness and failures over (candidate, result) pairs in FR-120's pair order, and declare no result constant for the disjunct (or one per step), or add a refusal code. | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:135-148, :177-187, :261-269; spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:451-455 | missing-requirement |
| FND-003 | medium | The candidate expansion has no refusal for a written field with no finite domain. Enabledness, deadlock terminality and postcondition failures expand "each value of its domain". FR-120's value domains leave every other type, for example `Integer` (FR-120's `id: Integer`), unbounded. That type still has an SMT form (`Int` with no range), so `field-sort` does not refuse it, and `parameter-domain` covers parameters only. The `candidates` measure then has no finite value to compare with its limit. Fix: refuse such a field (a new code, or extend `field-sort`/`parameter-domain`) whenever the query expands an application that writes it, at the field. | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:101-102, :177-187, :346 | missing-requirement |
| FND-004 | medium | Under false-extension, model-side failures are not restricted to live copies. The atom bullet limits failures to "a live copy". The precondition and postcondition failure bullets do not, and a copy past the end is "constrained only by its range assertions". A past-end copy can therefore take any in-range state, including an unreachable one, where a guard or postcondition with a partial construct (a division, a `value` read) is undefined. The V-2 query is then sat with no real counterexample. No `BoundedComplete` proof can be obtained, and the candidate never replays. Fix: conjoin `(not h<p>)` to every failure at `p` under false-extension, and state it in the Undefined evaluation list. | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:192-193, :254-270 | wrong-requirement |
| FND-005 | medium | `Unrolling{depth}` with `depth < u` for `eventually[0,u] holds(I)` encodes "not yet" as a violation. The violation is `h<p>` or `not I` at every `p` in `0..=min(depth, u)`. When `depth < u`, a model in which `I` first holds at `depth+1..=u` satisfies it, so a sat answer is not a refutation. For `always[0,u]` the same query is a real bad prefix. The FR says only that V-2 reads `depth >= u`. It does not say that a sat `eventually` query below `u` is not a counterexample candidate, and it does not refuse it. Fix: refuse `query-kind` for `eventually[0,u]` at `depth < u`, or state that such a sat answer carries no outcome. | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:197-202, :279-281 | missing-requirement |
| FND-006 | medium | The printed form of the false-extension step assertion is ambiguous. "`h<i+1>` equal to `h<i>` or copy `i` terminal" reads as `(or (= h<i+1> h<i>) T)` or as `(= h<i+1> (or h<i> T))`. The two are equivalent but print different bytes. Nor is it pinned how "copy `i` terminal" prints over several applications: `(not (or e1 ... en))` or `(and (not e1) ...)`. AC-17 and TC-900 step 9 assert this shape, and #1287's script printer has to choose one. Fix: give the exact s-expression for the step assertion and for terminality. | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:186-193, :652 | wrong-requirement |
| FND-007 | medium | "FR-120's candidate order" is not defined. FR-120 says only that candidates are generated "in canonical order". Its stated orders are contract findings by transition-identity bytes, then candidate state-key bytes. A byte order of keys differs from domain order once a range passes 9 (`10` sorts before `2`). AC-17 assumes domain order (candidates `0, 1, 2, 3`). Enabledness, terminality and postcondition-failure disjuncts are all printed in this order, so a coder must guess. Fix: define the candidate order in FR-315: written fields in (object, field) order, the first most significant, each in FR-120's value-domain order, absent first. | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:180-186, :264-266; spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:458, :488-490 | wrong-requirement |
| FND-008 | medium | The consequences of the unconditional stutter are not stated. Over FR-315-AC-1's `Counter`, consider a TP-3 item that is not stutter-invariant (ADR-018 IV-7), such as `always (holds(c.value = 0) implies eventually[0,1] holds(c.value = 1))`. The stutter at `0` gives a spurious base model at every depth, so EN-3 can never close for an item that holds. The FR says that replay refuses a stutter at a state that is not terminal. It does not say what CG's backend does with such a model. Replay that disagrees settles `inconclusive` `ReplayParity` (SM-7), which reports an engine defect where the over-approximation is working as intended. Fix: state that a model whose stutter step is taken at a copy that is not terminal is no counterexample candidate, and say how CG checks this (it evaluates terminality at stutter steps) and what it reports (V-6 or V-5, never `ReplayParity`). Or encode the stutter with an exact terminality conjunct for items outside the stutter-invariant fragment. | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:149-168 | missing-requirement |
| FND-009 | medium | Optional fields make distinctness and stutter equality ill-defined. Distinctness holds when "some value or presence constant ... differs". The value constant of an absent field is unconstrained, so two copies of one state, both with the field absent, count as distinct. The step case can then go around a loop of equal states, and induction may never close for a true item. The stutter's "every field equal" and the frame's equalities also do not say whether an absent field's value constant is equated. That changes both the models and the printed bytes. Fix: compare an optional field as presence equal and (absent, or values equal), and say the same for stutter and frame equality. | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:121-126, :149-151, :283-285 | wrong-requirement |
| FND-010 | low | Some printed orders are still not pinned: the stutter's conjunct order and its `(= s<i+1>.x s<i>.x)` orientation, which only AC-15 fixes; where the deadlock item's P-undefined failure stands among the negated property's disjuncts; and the conjunct order inside item 7's per-position assertions. Fix: state each one in Canonical printing. | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:149-153, :270, :291-298, :321-324, :650 | wrong-requirement |
| FND-011 | low | For a TP-2 item at `depth > u`, the negated property admits atom definedness failures at positions `u+1..=depth`. UE-1 says a TP-2 `on origin` claim reads positions `0` to `h` only. "An atom the claim reads at `p`" implies the bound, but the Negated property bullet gives the range `0..=depth` for failures and `0..=min(depth, u)` for violations only. Fix: bound the claim's atom failures by `min(depth, u)` there too. | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:255-256, :279-281 | wrong-requirement |
| FND-012 | low | The refusal codes do not close over `SmtEncodingRefusal`. Outputs says "each refusal has one code", but the refinement refusals (`any` row, `_` argument, a mapping row outside `QF_LIA`) name no code. The hyper codes (`hyper-form`, `hyper-body`, `subject-binding`, `match-conjunct`, `product-identity`) are outside the catalogue table. `instance-binding` means an object that does not conform to its type in the table, and a malformed `instance` in Product identity. Fix: give each refinement refusal a code, and list the full code set of the type once. | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:88-108, :378-385, :517-531, :603-615 | missing-requirement |
| FND-013 | low | The precedence of `too-large` over the other codes rests on an unstated build order. "At the points Size limits names" and "before it builds the terms the measure counts" do not place, for example, the `constants` check relative to `several-instances` or `out-of-fragment`. AC-21's one vector has no competing check. `encode_hyper_query` has no size limit, so its "this requirement's ... refusals" leaves open whether `too-large` applies. Fix: place `too-large`'s measures as rows of the ordered table, and state whether hyper queries carry them. | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:110-115, :335-351, :392-394 | wrong-requirement |
| FND-014 | low | Some vectors are not pinned. AC-23 gives its TP-3 item with "such as", and AC-21's `several-instances` vector inherits that item. AC-17's `query-kind` vector is "`InductionBase{depth: 3}` for the claim", which is `always[0,5]`, while TC-900 step 9 encodes it for `always[0,3]`. Fix: name one item in AC-23 (with its `over (c: Counter)`), and use one claim in AC-17 and TC-900 step 9. | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:652, :656, :658; spec/test-cases/TC-900-the-smt-lib-encoding-is-canonical.md:41-42 | wrong-requirement |

## Verdict

Not mergeable yet. The core soundness arguments hold. These are the
infinite-trace over-approximation, the exact h-flag termination behind
FR-314-AC-1's V-2 vector, the shortest-path coverage of definedness by base
and step, the reading of failures at `depth`, and DL-7. FND-001 is an AC
that contradicts FR-125's canonical instance order. FND-002 to FND-009 are
gaps that would make #1287's coder guess:

- result-declaring operations;
- unbounded written fields;
- past-end failures;
- `eventually` below its horizon;
- the h-step printing;
- the candidate order;
- the outcome of a non-terminal stutter;
- optional-field equality.

FND-004 and FND-008 also cost proofs, and an engine-defect cause can be
reported where none occurred. None of the findings makes a `proved` result
unsound. Fix FND-001 to FND-009 in this PR. The lows may be fixed or
deferred. Merge origin/main (#1288) into the head before merging, and
retarget #1287's AC-4 test from `Slot` to `Tally`.

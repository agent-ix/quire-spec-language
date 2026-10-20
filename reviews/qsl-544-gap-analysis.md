---
id: SR-13363
title: "Gap analysis of QSL-544 part 2 (PR #1071) against FR-140, FR-141, FR-142, FR-144 (safety) and FR-145"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@1c3b7fae6; PR #1071 (git diff refs/remotes/origin/main...HEAD); spec/functional/FR-140, FR-141, FR-142, FR-144, FR-145 (all ACs); FR-135, FR-139, FR-143, FR-146, FR-149 (context); spec/decisions/ADR-020 section 8; spec/test-cases/TC-545, TC-546, TC-547, TC-549, TC-550, TC-555, TC-556, TC-1086; QSpec FR-375, FR-377, FR-379, FR-383 (quire-specification origin/main); Linear QSL-544, QSL-545, QSL-547, QSL-549, QSL-563, STD-211"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-140
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-141
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-142
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-144
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-145
    type: reviews
---
# SR-13363: Gap analysis of QSL-544 part 2 (PR #1071)

## Summary

Ticket: QSL-544 (E13), part 2. Tool versions: `quoin matrix --json` (quoin
0.28.3, quire 0.36.2, engine 0.50.2), exit 0, with
`QSPEC_DIR=/Users/peter/dev/quire-specification`. Plan completion: not
assessed.

Computed matrix at head 1c3b7fae6, for the 42 ACs of the five FRs in scope:

- **FR-140**: AC-1 to AC-3 and AC-5 to AC-14 are `tagged`. AC-4 is
  `untagged`.
- **FR-141**: AC-1 to AC-3, AC-5, AC-7 and AC-8 are `tagged`. AC-4 and AC-6
  are `untagged`.
- **FR-142**: AC-1 to AC-3, AC-6 and AC-7 are `tagged`. AC-4, AC-5 and AC-8
  are `untagged`.
- **FR-144**: AC-2 to AC-4 and AC-6 are `tagged`. AC-1, AC-5 and AC-7 are
  `untagged`.
- **FR-145**: AC-1, AC-3 and AC-4 are `tagged`. AC-2 and AC-5 are
  `untagged`.

I read the test behind every tagged AC. Code with no owning requirement:
none. Every new public function maps to FR-140/141/142/144/145. Stubs: none.
The not-built paths refuse with typed values: `ModelCheckRefusal::HiddenFields`,
`ModelCheckRefusal::RefinementLasso`, and `StepVerdict::Fault` for a hidden
field.

Below, each AC that is unbuilt or only partly built, with the ticket that owns
it. "Owned" means a named ticket whose Implements line or acceptance covers
the AC. "Named only" means the AC is assigned only in a PR body or comment.

- **FR-140-AC-4.** First half (RegisterHistory, history) is QSL-545, named
  only (FND-001). The second half, the hidden slot, is tested on
  CasRefinesCounter, not on RegisterHistory, and has no AC tag.
- **FR-140-AC-11.** The rule is built and tested on values. Writing the row as
  `none` / `Some(e)` needs an S2/S3 Option constructor. The QSpec side,
  STD-211 item 7, is Done. No QSL ticket exists (FND-004).
- **FR-140-AC-14.** Built, except the "even when an earlier row was undefined"
  case, which is untested (FND-006).
- **FR-141-AC-4, FR-142-AC-5, and the Coin part of FR-145-AC-1.** These need
  hidden fields and the AX-2 candidate set. QSL-545, named only (FND-001).
- **FR-141-AC-6, FR-142-AC-4, FR-142-AC-8, FR-145-AC-5 (TC-554), the `writes`
  part of FR-144-AC-3, and the history part of FR-145-AC-1.** These need
  history fields. QSL-545, named only (FND-001).
- **FR-141-AC-5 (protocol subject).** Also FR-142's `RefinementSubject::Protocol`
  and FR-145's `ProtocolSystem` re-execution. No ticket names them. QSL-563
  implements FR-147/148/214 but not FR-141 (FND-002). The matrix shows
  FR-141-AC-5 as `tagged` only because of a doc comment already on main
  (FND-002).
- **FR-144-AC-1 and FR-144-AC-5.** These need the requirement record, TP-5
  `PropertyForm::Refinement` and the obligation identity. Blocked on
  quire-semantic-value (`PropertyForm::Refinement`, `RefinementFailure`) and
  on S4 emitting the refinement node. No ticket exists (FND-003).
- **FR-145-AC-4.** The node-identity, stale-`package_id` and recompile
  refusals are blocked as above, with no ticket (FND-003). The
  RegisterHistory lasso case is QSL-545 plus QSL-547.
- **FR-144-AC-2.** The liveness half and the divergence variant are QSL-547,
  owned ("FR-144 (liveness)"). The safety half `Certified` value is QSL-549
  (FND-005).
- **The hidden-`ensure` part of FR-144-AC-3.** QSL-547, owned. It needs
  QSL-545 hidden fields first.
- **FR-144-AC-7 (TC-556).** QSL-549, named only. QSL-549's Implements line
  says "new certificate FR (AM10)" and does not name FR-149, FR-144-AC-7 or
  TC-556 (FND-005).
- **FR-145-AC-2, and the lasso parts of FR-145-AC-3.** QSL-547, owned
  ("FR-145 (lasso)").
- **FR-142 "retain every explored edge for FR-143".** QSL-547, owned.

No AC was dropped without being mentioned: the PR body and the QSL-544
comment of 2026-10-19 name every gap above. But FND-001 to FND-004 list the
ACs that no ticket's acceptance owns. Under each ticket's "nothing beyond
them" rule, they will not be built unless their tickets name them.

## Scope examined

| Unit | Role | Excerpt |
| --- | --- | --- |
| FR-140-AC-4 | examined | "Over `RegisterHistory` (FR-138's fixture), the concrete state `value = 1` with `last = 0` maps to `value = 1`, `prev = 0`. With the `prev` row removed, the same state maps to `visible` with `value = 1` and `hidden = [(r, prev)]`." |
| FR-140-AC-11 | examined | "With the abstract `Queue` declaring `head: Int[0, 1]?` and the row `head = if self.size >= 1 then Some(self.head) else none` ..." |
| FR-140-AC-14 | examined | "A broken evaluator invariant while reading a row returns `MappingFailure::Fault` with its catalog code at once, even when an earlier row was undefined, and never `Undefined` or `Undetermined`." |
| FR-141-AC-4 | examined | "Fixture `Coin` ... After concrete `toss` the candidate set holds two states, `side` 0 and `side` 1 ..." |
| FR-141-AC-5 | examined | "Over FR-136-AC-4's protocol subject with branch `A`'s attempt node mapped `-> any` and `incA` mapped `-> Spec::Counter::inc(self)` ..." |
| FR-141-AC-6 | examined | "Over `RegisterHistory` with FR-138's `inv` update, `check_step` on `write(r, 0)` from the initial state returns `Undefined` with `where` position 1 ..." |
| FR-142-AC-4 | examined | "`RegisterHistory` (FR-138) returns `Holds{Exhaustive}` with 4 product states ..." |
| FR-142-AC-5 | examined | "`Coin` (FR-141-AC-4) with `side` hidden returns `Holds{Exhaustive}` ..." |
| FR-142-AC-8 | examined | "`RegisterHistory` with FR-138's `inv` update returns `Violated` with prefix `write(r, 0)` ..." |
| FR-144-AC-1 | examined | "`CasRefinesCounter` writes one requirement record (`temporal-satisfaction`, `PropertyForm::Refinement{liveness: true}`) ..." |
| FR-144-AC-2 | examined | "`CasRefinesCounter` settles `proved`, `closed-scope`, `Proved{basis: Exhaustive, certification: Uncertified}`, method `explicit-state`, safety half `Proved{basis: Exhaustive, certification: Certified}` ..." |
| FR-144-AC-5 | examined | "Two `CasRefinesCounter` requests that differ only in the abstract subject's initial snapshots ... have different obligation identities" |
| FR-144-AC-7 | examined | "`CasRefinesCounter` with its `ensure` row removed settles `proved`, `closed-scope`, `Certified`, after FR-149 accepts the certificate ..." |
| FR-145-AC-2 | examined | "FR-143-AC-1's divergence lasso reproduces `Divergence{constraint: inc}` ..." |
| FR-145-AC-4 | examined | "Refusals settle no result: an envelope naming another refinement node (`stale_dependency`/`content-mismatch` naming both identities) ..." |
| FR-145-AC-5 | examined | "FR-142-AC-8's counterexample replays to `reproduced-with-evaluated-witness` ..." |
| FR-140-AC-1..3, AC-5..10, AC-12, AC-13 | examined | tagged tests read; each asserts its AC (clean) |
| FR-141-AC-1..3, AC-7, AC-8 | examined | tagged tests read; each asserts its AC (clean) |
| FR-142-AC-1..3, AC-6, AC-7 | examined | tagged tests read; AC-2's "last step lowers value from 2 to 1" and AC-7's rerun of the `any` request are not asserted (minor) |
| FR-144-AC-4, AC-6 | examined | tagged tests read; each asserts its AC (clean) |

## Verdict

**The build is complete for what the PR claims, but four ownership gaps need
the planner before merge-and-close.** The built ACs are backed by tests that
assert them, except the over-claiming tags recorded in SR-13362 (FND-005,
FND-006). QSL-544 should not close until each unbuilt AC above is named in the
acceptance of an open ticket.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Several ACs are deferred to QSL-545, but QSL-545 does not own them: FR-141-AC-4, FR-141-AC-6, FR-142-AC-4, FR-142-AC-5, FR-142-AC-8, FR-145-AC-5 (TC-554), the RegisterHistory half of FR-140-AC-4, the history and Coin parts of FR-145-AC-1, and the `writes` part of FR-144-AC-3. QSL-545's Implements line names FR-138 only, and its acceptance says "the tests in this PR back FR-138 and pass; nothing beyond them". So by its own acceptance these FR-140/141/142/144/145 ACs are out of its scope, and no ticket owns them. Add them to QSL-545's Implements/acceptance, or file the AX-2/history-wiring slice. | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md:192,194; spec/functional/FR-142-check-a-refinement-s-safety-half-on-the-explicit-state-product.md:158-159,162 |
| FND-002 | medium | No ticket owns FR-141-AC-5 (protocol subject), FR-142's protocol concrete subject (`RefinementSubject::Protocol`) or FR-145's `ProtocolSystem` re-execution. The QSL-544 comment says "not built" and names no ticket. QSL-563 (E15b-ref) implements FR-214, FR-147 and FR-148 but not FR-141/142/145. The matrix also reports FR-141-AC-5 as `tagged`, because a doc comment on main (`qsl-semantics/tests/it/variant_fields.rs:1669`, "FR-360-AC-6, FR-141-AC-5") is harvested as a binder. That pre-existing line makes an unbuilt AC look covered. | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md:193; qsl-semantics/tests/it/variant_fields.rs:1669 |
| FND-003 | medium | No ticket exists for FR-144-AC-1 (requirement record, `PropertyForm::Refinement`, deadlock item), FR-144-AC-5 (obligation identity), or FR-145-AC-4's node-identity, stale-`package_id` and recompile refusals, nor for the FR-145 facade around `replay_refinement_trace`. The PR names the blockers: the pinned quire-semantic-value has no `PropertyForm::Refinement` or `RefinementFailure`, and S4 refuses a refinement as not emitted (`emit_unwritten_refinements`). A search of Linear (ix-board search for PropertyForm::Refinement, RefinementFailure, emit_unwritten_refinements, refinement emission) found no ticket. These ACs are blocked with no owner. File the quire-semantic-value change and the S4 refinement-node emission. | spec/functional/FR-144-request-and-settle-a-refinement-item.md:129,133; spec/functional/FR-145-replay-a-refinement-counterexample.md:125 |
| FND-004 | low | FR-140-AC-11's row spelling (`if ... then Some(self.head) else none`) needs an S2/S3 Option constructor. STD-211 item 7 asked for it on the QSpec side and STD-211 is Done, but no QSL ticket builds the production. The rule is tested only on values in `map.rs`. The QSL-544 comment says "a QSpec Option-constructor ticket (planner)" without an id. | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md:212; qsl-replay/src/refinement/map.rs:572 |
| FND-005 | low | QSL-549 owns the safety-half `Certified` settlement (FR-144-AC-2's safety half, FR-144-AC-7, TC-556). But its Implements line reads "new certificate FR (AM10)" and names neither FR-149 nor those ACs, so the link from the deferred ACs to the ticket exists only in PR prose. Name FR-149, FR-144-AC-2 (safety half), FR-144-AC-7 and TC-556 on QSL-549. | spec/functional/FR-144-request-and-settle-a-refinement-item.md:130,135 |
| FND-006 | low | Two built ACs have untested clauses. FR-140-AC-14's "even when an earlier row was undefined" is untested; the coder says there is no public way to inject a fault between two rows. The second half of FR-140-AC-4 (the hidden slot) is tested only on CasRefinesCounter and carries no AC tag. A `#[cfg(test)]` seam on `Outcomes` plus `?` ordering, like the existing AC-13 unit test, could cover the first. | qsl-replay/tests/it/refinement_mapping.rs:115,241 |
| FND-007 | low | No ticket owns `UndefinedReason::OnlyNotOne`, which still carries `found` (a match count). FR-139 now says the payload is binding, locus and enclosing row, with no count. The QSL-544 comment calls it "QSL-546's implied change", but QSL-546 is Done, so the change is unowned. It does not affect this PR's ACs, because FR-140-AC-9 reads only the reason. | spec/functional/FR-139-type-and-evaluate-population-valued-expressions.md |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | medium | Four ACs that #1072 merged into this PR's FRs are unbuilt and not named anywhere: not in the PR body's "Left out" list, not on QSL-544, and not on any other ticket. FR-141-AC-9 (`HeavyEnq`): only the zero-meter `check_step` unit case exists, untagged; the FR-142 run ending `Stopped{EvaluationMeter, B}` and FR-144's V-7 settlement are untested. FR-141-AC-10 (`Reads`): only `Soft` unit tests; no fixture for the receiver, argument, history and post-row combinations, the swapped-system Fault case included. FR-141-AC-11 (`Coin3`, `abstract_successors` counts): the `commitB` count half can be built now; the `Coin` half needs QSL-545. FR-145-AC-6: unbuilt, and the code contradicts it (SR-13362 FND-013). The matrix reports all four as `untagged`. Name an owner for each (QSL-544 for the first three and AC-6; QSL-545 for `Coin3`'s hidden half), or build them in this PR. | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md:243-245; spec/functional/FR-145-replay-a-refinement-counterexample.md |

## Dispositions

Round 1, at head 636cb4e43. The ticket state was read from Linear on 2026-10-20. The planner could not create new issues, so ownership was recorded as scope comments on the existing tickets. The recomputed matrix now shows FR-140-AC-11, FR-140-AC-14, FR-144-AC-3, FR-145-AC-1, FR-145-AC-3 and FR-145-AC-4 as `untagged`, which is correct now that their partial tags were removed.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | A QSL-545 comment (2026-10-20T00:16:19Z) widens its scope to FR-141-AC-4/6, FR-142-AC-4/5/8, FR-145-AC-5 (TC-554), the first half of FR-140-AC-4, and the history/hidden parts of FR-145-AC-1 and FR-144-AC-3, and says "Done only when those land". |
| FND-002 | fixed | A QSL-563 comment (00:16:20Z) adds FR-141-AC-5 and the protocol paths of FR-142 and FR-145. The doc-comment binder already on main (`qsl-semantics/tests/it/variant_fields.rs:1669`) still makes the matrix show FR-141-AC-5 as tagged. That line is not in this diff; it should be cleaned up under QSL-563. |
| FND-003 | still-open | Still no owner for FR-144-AC-1, FR-144-AC-5, FR-145-AC-4's node-identity, stale-`package_id` and recompile refusals, or the FR-145 replay facade. The blockers are quire-semantic-value `PropertyForm::Refinement`/`RefinementFailure` and S4 emission of the refinement node. No scope comment on QSL-544, QSL-545, QSL-547, QSL-549, QSL-563, QSL-905, QSL-546 or QSL-548 names them. This blocks closing QSL-544, not the code in the PR: one planner scope comment naming the owning ticket clears it. |
| FND-004 | fixed | A QSL-544 comment (00:16:22Z) names QSL-544 itself as the holder for FR-140-AC-11's `none`/`Some` row spelling and keeps it open. |
| FND-005 | fixed | A QSL-549 comment (00:16:23Z) names FR-149, FR-144-AC-7 and TC-556. The flip of FR-144-AC-2's safety half is recorded in `refinement_settle.rs`'s module comment ("The certificate's ticket flips this row"). |
| FND-006 | deferred | FR-140-AC-14's "fault even after an earlier undefined row" stays untested and is no longer tagged. The behaviour holds by construction: every fault path in `map_state`/`map_object` returns through `?` at once, and `Outcomes::record` drops `Fault`, so no earlier outcome can win over it. I judge a test-only fault hook not needed before merge, since the AC is honestly untagged and QSL-544 stays open as its owner. When QSL-544 next lands code, add a `#[cfg(test)]` seam on `map_object` that fails the second object's row, or reach a lift fault through a fixture reference of an unkeyable type. The untagged second half of FR-140-AC-4 is covered, and QSL-545 now owns the first half. |
| FND-007 | fixed | A QSL-546 comment (00:16:27Z) reopens it to drop `UndefinedReason::OnlyNotOne`'s stale `found` payload. |
| FND-003 | fixed | Round 2 (f572dc746): A QSL-544 planner comment (2026-10-20T00:47:37Z) names QSL-544 as owner of FR-144-AC-1, FR-144-AC-5, FR-145-AC-4's node-identity, stale-`package_id` and recompile refusals, and the replay facade, blocked on a coordinated quire-semantic-value PR plus a QSL pin bump. The PR body's "Left out" list still attributes them to QSL-918, an unrelated quire-semantic-value doc-comment ticket; correct the PR body to say QSL-544. |

Round 2 correction, at head f572dc746: my round-1 note said the recomputed matrix showed the partial ACs as `untagged`. I had not rerun the matrix in that round, and the note was wrong. `quoin matrix` at f572dc746 shows FR-140-AC-11, FR-140-AC-14, FR-144-AC-3, FR-145-AC-1, FR-145-AC-3 and FR-145-AC-4 still `tagged`, through AC ids in the tests' doc comments (SR-13362 FND-014). FR-140-AC-15, FR-141-AC-9, FR-141-AC-10, FR-141-AC-11, FR-144-AC-2 and FR-145-AC-6 are `untagged`.

---
id: SR-13470
title: "Code review of quire-spec-language PR #1094: model-side partial-order reduction (QSL-564, E12c)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@6fc8961654f89a1ae3159b5bac84886252a4db18; PR #1094, git diff refs/remotes/origin/main...6fc896165: qsl-analyze/src/model_check/{ample,check,graph,liveness,mod,product}.rs, qsl-eval/src/{model_subject.rs,simulation/{explore,footprint,mod,model}.rs,simulation/model/{footprints,frames,walk}.rs}, qsl-replay/src/{lib,outcome,proof_result}.rs, qsl-replay/src/model_check/{outcome,settle}.rs, qsl-semantics/src/check/footprint.rs, qsl-analyze/tests/it/*, qsl-eval/tests/it/footprints.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-155
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-156
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-157
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-160
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-162
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-021
    type: reviews
---
# Code review of quire-spec-language PR #1094

## Summary

Ticket: QSL-564. Code review with the rust-review lane folded in. The diff is
35 files, +4357/-280: ample sets (FR-156), fairness visibility (FR-157), the
state-free `TransitionSystem::footprint` hook and `ModelLetters::reads`
(FR-162), POR settling (FR-160), and the reduced-versus-unreduced differential
(TC-886).

Read in full: QSL FR-155 to FR-162, ADR-021 §1.2 (POR-1 to POR-11) and §2,
QSpec FR-383 (quire-specification@240faeb, "Locations", "Independence and
visibility"), and the TC files named in the brief. Reviewer evidence lives in
scratch only and was not pushed:

- **Abstract brute-force oracle**
  (`sr-1094/oracle/por_oracle.py`, `gen2.py`, `deadlock.py`, `handcase.py`).
  It mirrors `ample.rs` (visibility, the stubborn-set closure, choice, C3 on
  discovered states) over random object models. The models have field writes,
  receiver-membership reads, `exists`-over-population guards, creates,
  receiver and type-wide deletes, and every-object writes. It compares full
  and reduced verdicts for safety, deadlock freedom and liveness under weak
  and strong fairness, `whole` and `each`. Over about 22,000 random models
  (about 138,000 full-versus-reduced verdict pairs per variant, over
  500,000 reduced expansions), the PR's `affects` relation had 0
  mismatches. The spec's symmetric `meets` also had
  0. The oracle does catch planted bugs: with fairness visibility removed it
  found about 20 wrong `Holds` per 1,500 models, and with the disabled-member
  closure removed it found wrong safety verdicts. It also reproduces FR-157:
  under `affects` it picks `A = {toggle}` at the Gate's initial state, and
  with the fairness rule removed it picks `{arm}` and returns the wrong
  `Holds`. Under `meets` it picks `A = E(s)`, so the Gate is not reduced at
  all.
- **Real-engine probes** (`sr-1094/probe/sr_probe.rs`, built into a scratch
  copy of `qsl-analyze/tests/it`). These back FND-001 and FND-003.
- **Mutations of the real code**
  (`sr-1094/mutate.py`, log `sr-1094/mutate.log`):
  - M1, fairness visibility off: 6 tests fail, among them the differential.
  - M2, disabled-member closure off: 3 §7.6 vectors and the differential
    fail. The FR-156-AC-4 test stays green (FND-004).
  - M3, `affects` replaced by `meets`: the 8 FR-157 Gate tests fail. The
    differential stays green, as it should: `meets` only reduces less.
  - M4, C2 off: 11 tests fail, the differential among them.
  - M5, C3 off: 3 tests fail, the differential among them.
  - M6, dependence closure off: 6 tests fail, the differential among them.

**The key question: `Location::affects` versus the spec's symmetric
`meets`.** `affects` is sound, and the spec text has to change. QSpec FR-383
and ADR-021 POR-4 make `Membership{P, k}` meet every `Field{k, f}` in both
directions. POR-11's cell model lets a location stand for every cell it
meets, so a read of `Membership{gates, g}` would also read every field cell
of `g`. Commutation (POR-11 step 2) and Peled's C1 only need one fact: a
step's writes change no cell that the other step's enabledness or effect
reads. A field write changes only its own field cell, never a membership
cell. A creation or deletion of `k` does change `k`'s field cells. So the
exact relation is directional:

- a written `Membership` covers the membership cell and every field cell of
  `k`;
- a read `Membership` covers the membership cell only.

That is what `affects` implements. Write/write pairs keep plain `meets`, which
is conservative. No case in the oracle needs the field-write-to-membership-read
meet. Read-side rules always pair a field read on an object the transition
does not name with an any-object read, and the receiver's own membership is
always in `R(t)`. Under the literal symmetric meet, FR-157-AC-1, AC-2, AC-3
and AC-5 and ADR-021 §7.5 cannot hold, because `toggle` then meets `go(g)`'s
enabling footprint (mutation M3). The merged spec contradicts itself.
FND-002 records the spec follow-up.

**The coder's list of deviations from the spec**, ruled one by one:

- FR-156-AC-2 path order: spec follow-up (FND-005).
- `arm2` sorting before `arm`: spec follow-up (FND-006). The code follows
  QSpec FR-181's identity-byte order.
- Absolute state counts: spec follow-up (FND-007).
- §7.6's type-wide delete with `cancel` left unconstrained: accepted. S3
  rejects `M::jobs` outside a refinement. The vector still refutes by
  `boom(j)`, and the test says what it does not claim.
- The unreduced liveness `Holds` counted through
  `ComponentCertificateNotBuilt`: accepted. That refusal is reached only after
  the whole product is explored with no fair accepting component, so it is a
  `Holds` with no certificate. The test matches that variant by name. No
  finding.

Gates, run in my own clone and target directory, all green:

- `cargo test -p qsl-eval -p qsl-replay -p qsl-analyze -p qsl-semantics`
- root `cargo test --test it` (975 passed, 6 ignored)
- `cargo clippy --workspace --all-targets -- -D warnings`
- `make string-edge`
- `make arch-lint-api-surface-qsl`
- `make arch-lint-canonical-encoder`
- `make arch-lint-qualified-core`

## Verdict

Changes requested. Two high, two medium and six low findings.

What is correct:

- C2, C3 on product states with the breadth-first proviso, and the C0
  singleton shortcut (mutations M4 and M5, and the oracle).
- Ample-set determinism: seeds in canonical identity order, footprints cached
  by identity.
- Fairness visibility for `whole` and `each`, weak and strong (M1 and the
  oracle). The filter reads the full `Enabled` set.
- `ReductionHorizon`: completing to `max_depth` under POR settles
  `inconclusive`/`reduction-horizon`; symmetry alone settles `BoundReached`;
  `ConstraintReached` takes precedence.
- Every POR proof settles `Proved{Reduced{[..., PartialOrder{BreadthFirstRevisit}]},
  Uncertified}`, in the order symmetry then partial order.
- The differential corpus works: it catches M1, M2, M4, M5 and M6.
- The qsl-semantics `Reader`/`BodyScope` refactor preserves the derivation.
- Every new test tag names a test that asserts its AC, except FR-156-AC-4
  (FND-004).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The ample-set candidate is built only from identities over objects the state holds. `identities_of` walks `walk.rows`, the objects present, and `domains_of` draws argument domains from `walk.references()`. FR-155 says otherwise: "the partial-order closure asks for the enabling footprint of every disabled transition, among them transitions whose objects no state holds yet". FR-162 also requires the hook for every identity "whether or not the objects it names exist", for disabled candidate members, and FR-155-AC-7 says "the candidate seeded by spawn holds the disabled bump(d)". Real-engine probe (`sr_probe_ample_at_start`): `x: Job` has `cancel()` (deletes self), `y: Maker` has `spawn()` (creates Item) and `probe(it: Item, j: Job)`, and the universe `items = {d}` starts empty. The candidate seeded by `cancel(x)` is `{cancel(x)}`. It leaves out `probe(y, d, x)`, which reads `Membership{jobs, x}`, so with every successor fresh `Por::ample` returns `{cancel(x)}`. The abstract oracle (`handcase.py`) turns the same shape into a missed deadlock (full: deadlock; reduced: none). With universe identities the deadlock is found again. The real engine is saved today only by accident: a permissive frame gives every field-free transition a no-op successor, and C3 rejects it. Fix: enumerate identities over the population universes, which the state-free footprints already support. Add a candidate-level test for FR-155-AC-7. | qsl-eval/src/simulation/model/walk.rs:218-238; qsl-eval/src/simulation/model/frames.rs:84-96; qsl-analyze/src/model_check/ample.rs:230-240 |
| FND-002 | high | The code breaks merged spec text. FR-155 "Independence" reads: "`independent(t, u)` SHALL be true exactly when no location of `t.writes` meets one of `u.reads ∪ u.writes`". QSpec FR-383 and ADR-021 POR-4 make `Membership{P, k}` meet `Field{k, f}` in both directions. `independent`, the visibility rule and the closure all use `Location::affects`, under which a field write does not meet a membership read. The code is right and the spec is wrong: the symmetric meet makes FR-157-AC-1/2/3/5 and ADR-021 §7.5 impossible (mutation M3), and the directional relation is sound (cell argument and oracle in Summary). A spec follow-up must make the relation directional before or with this merge, in five places: QSL FR-155 "Meeting" and "Independence", FR-156 "Visibility" and "Candidates", FR-157 "Fairness visibility", ADR-021 POR-4 and the POR-11 cell mapping, and QSpec FR-383 "Locations" and "Independence and visibility". The new rule: a written membership covers the object's field cells; a read membership covers only the membership cell. Separately, the doc comment on `affects` cites POR-4, which states the opposite. | qsl-eval/src/simulation/footprint.rs:45-56; qsl-eval/src/simulation/footprint.rs:144-150; spec/functional/FR-155-derive-and-enforce-read-and-write-footprints.md:129-135 |
| FND-003 | medium | When FR-155's derivation cannot give an operation a footprint, the POR run ends in an internal fault, not a refusal. The derivation refuses some forms that FR-383 has rules for, such as a function call or a navigation in a `pre` or `post`. `TransitionSystem::footprint` maps every `FootprintError` to `BROKEN`, and so does `successors_enforced` (`WithinStop::Footprint`). Probe `sr_probe_operation_footprint_refusal`: `spin()` with pre `Noop()` and claim `always holds(not x.f)`. Unreduced, it returns `Violated`. Under POR it returns `Err(Fault(InternalFault{stage: "model-check", invariant: "model-expansion-invariant"}))`. A real violation is lost, and a known capability gap is reported as a broken invariant. Atom and constraint refusals already refuse up front (`ModelCheckRefusal::Footprint`). Derive every operation footprint at `Por::new` or the pre-check and refuse the same way. | qsl-eval/src/simulation/model.rs:985-996; qsl-eval/src/simulation/model/footprints.rs:533-542 |
| FND-004 | medium | FR-156-AC-4's test does not exercise the disabled-member closure. With that closure removed (mutation M2), `the_candidate_reaches_through_a_disabled_member_and_the_choice_is_deterministic` still passes: `arm` is the seed, and `arm`'s own dependents (`arm2` reads `c`, `go` reads `c`) bring in `arm2` and `go`. Its `enabled_part` assertion is circular: it filters the enabled set by name, not the candidate. The AC text has the same weakness, since "seeded by any transition dependent on `arm2`" is satisfied by `arm` itself. Fix the AC and the test together: use a seed that reaches `arm` only through a disabled member, and assert the candidate's enabled part from `candidate_seeded_by`. | qsl-analyze/tests/it/partial_order.rs:545-588; spec/functional/FR-156-expand-ample-sets-with-the-breadth-first-proviso.md:101 |
| FND-005 | low | FR-156-AC-2 says the deadlock prefix "is the path of AC-1's table" (`b, b, c, c, a, a`). With no visible transition, FR-156 "Choice" breaks the tie among singletons by seed order, so `bump(a)` wins and the path is `a, a, b, b, c, c`. The AC contradicts its own requirement and needs a spec follow-up. The test checks only the length (6) and the end state, so it pins neither path. It should assert the operations. | qsl-analyze/tests/it/partial_order.rs:316-358; spec/functional/FR-156-expand-ample-sets-with-the-breadth-first-proviso.md:99 |
| FND-006 | low | FR-157's AC preamble (and ADR-021 §7.5) gives "Canonical transition order is `arm`, `arm2`, `go`, `toggle`". FR-101's order is QSpec FR-181's identity-byte order. In those bytes, the operation text inside the RFC 8785 identity is escaped, so `arm` is followed by `\"` (0x5C), which sorts after `2` (0x32): `arm2` comes before `arm`. The code is right. Spec follow-up: no AC verdict depends on the order, but the preamble states it as fact. | spec/functional/FR-157-make-fairness-enabling-writes-visible.md:88-89 |
| FND-007 | low | Several ACs give absolute counts ("over 4 states", "over 6 states", "over 5 states"): FR-156-AC-5 and AC-6, FR-155-AC-6, FR-157-AC-5. No test asserts them. `vector()` asserts only that the reduced and unreduced runs agree on operations, kind and `product_states`. That checks the property that matters, but leaves the counts unverified. Either spec FR-160 and ADR-021 §7.6 define what "states" counts, or the tests pin the observed counts. | qsl-analyze/tests/it/partial_order.rs:831-875; spec/functional/FR-156-expand-ample-sets-with-the-breadth-first-proviso.md:102-103 |
| FND-008 | low | `Por::visible` and `Por::candidate_seeded_by` are `pub` methods of the shipped `qsl_analyze::model_check` API, but only the tests call them; production calls only `Por::ample`. That is test-only public surface in a library crate. Either make them test hooks the repo's way, or justify them as API. | qsl-analyze/src/model_check/ample.rs:190-227 |
| FND-009 | low | The safety engine evaluates every successor's monitor twice under POR. `Run::ample` calls `self.advance` on every successor to judge C3. `create` then calls `advance` again on each step the loop keeps, so every atom is evaluated twice, with fresh meters, at each reduced expansion. Pass the `(after, bad)` pairs from `ample` to the loop. | qsl-analyze/src/model_check/check.rs:1066-1115; qsl-analyze/src/model_check/check.rs:1142-1143 |
| FND-010 | low | FR-157-AC-3's test checks the reduced lasso entry (2) and position count (5), and the unreduced lasso entry (0) and count (3). It never checks the operations. The AC names the stem `arm, arm2` and the loop `toggle, toggle`, and any 2-step stem with a 2-step loop passes. | qsl-analyze/tests/it/partial_order.rs:612-641 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-011 | medium | The test now tagged FR-156-AC-4, `the_candidate_holds_the_writers_of_a_disabled_members_enabling_footprint`, does not test the scenario the AC states in companion spec PR #1096. The test adds `setd()` (pre `not self.d`), seeds the candidate with `arm`, and asserts candidate `{arm, arm2, go, setd}` with enabled part `{arm, setd}`. The AC as amended in #1096 adds `touch()` (framed `modifies self.d`, no pre), seeds with `touch`, and states enabled part `{touch, arm}`. Both scenarios do exercise the closure: with it removed, the test fails (mutation M2) and the oracle gives `{touch}`. But once both PRs merge, the tag would claim a different vector from the AC's. Align the test with the AC (or the AC with the test) before the pair merges. | qsl-analyze/tests/it/partial_order.rs:594-628; qsl-analyze/tests/it/por_world.rs:697-710 |
| FND-012 | low | The rewritten doc comment on `Location::affects` ends "POR-4 and QSpec FR-383 state the meet symmetrically, which makes every operation on one object depend on every other". Companion spec PRs #1096 and #299 make POR-4 and FR-383 directional, so once they merge the comment contradicts the spec it cites. Cite the directional POR-4 and FR-383 rule instead. | qsl-eval/src/simulation/footprint.rs:45-53 |

## Dispositions

Round 1, reviewed at `4afad2ca6e5572595130bdc82f936cfc0642793a`.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | 4afad2ca6: `identities_of` adds `absent_rows`, one row per universe key the state does not hold, at each concrete type the key's population admits. The rows also feed argument domains. Probe: the candidate seeded by `cancel(x)` now holds `probe(y, d, x)`, so `A(s) = E(s)`. Mutation (rows removed): the new test `the_candidate_holds_transitions_over_objects_no_state_holds_yet` fails. Scale, release build, with an `Item` universe of n keys and a two-parameter operation over items: n=64 gives 4,226 identities, listed in 40 ms, and `ample` takes 110 ms. That is the size the identity set already reaches when every key is held, and `max_candidates` still bounds it. No blow-up beyond the universe's size. |
| FND-002 | fixed | Spec side: companion PRs #1096 and #299 make POR-4, POR-11 and FR-383 directional (SR-13530 and SR-13531, both fixed in round 1). |
| FND-003 | fixed | 4afad2ca6: `Por::new` calls `check_footprints_derivable` first. Probe: unreduced returns `Violated`; POR returns `Err(Footprint(Refused(UnsupportedForm{clause: "SpinP", form: "function call"})))`. Mutation (check removed): `an_operation_with_no_footprint_rule_refuses_partial_order_reduction` fails. |
| FND-004 | fixed | 4afad2ca6: the circular `enabled_part` assertion is gone. The new test reads the enabled part from `candidate_seeded_by` and fails with the disabled-member closure removed (mutation M2). Scenario alignment with the AC is FND-011. |
| FND-005 | fixed | 4afad2ca6: the FR-156-AC-2 test asserts the path `a, a, b, b, c, c`, and #1096 states the same path. |
| FND-006 | fixed | Spec side: #1096 states QSpec FR-181's escaped-byte order, `arm2` before `arm`. |
| FND-007 | fixed | Spec side: #1096 defines the count as model states in the model-level reduced search. I checked each stated count against the engine: 8, 6, 4, 5, 6 and 6, all agree. The tests still assert reduced equal to unreduced rather than absolute counts. |
| FND-008 | fixed | 4afad2ca6: `Por::visible` and `Por::candidate_seeded_by` are `#[cfg(feature = "test-support")]`. The crate enables the feature for its own integration tests through a dev-dependency on itself (+1 Cargo.lock line). `test-support` is the repo's convention (qsl-cst, qsl-package, qsl-semantics, quire-exact). This is the first crate to enable it on itself, but it is the standard Cargo way to reach an integration-test hook, and `cfg(test)` does not reach `tests/`. Resolver 2 limits it to test builds. `no_shipped_dependency_enables_test_support` (root `cargo test --test it`), `arch-lint-api-surface-qsl` and `clippy -p qsl-analyze --all-features` all pass. Acceptable. |
| FND-009 | fixed | 4afad2ca6: `Run::ample` returns the `(after, bad)` reading of each kept step, and `create` uses it in place of reading again. The readings stay in step order through the `kept` flags. |
| FND-010 | fixed | 4afad2ca6: the FR-157-AC-3 test asserts the operations: reduced `arm, arm2, toggle, toggle`, unreduced `toggle, toggle`. |

Re-run at 4afad2ca6, all green:

- `cargo test -p qsl-eval -p qsl-replay -p qsl-analyze -p qsl-semantics`
- root `cargo test --test it`
- `cargo clippy --locked --workspace --all-targets -D warnings`
- `cargo clippy -p qsl-analyze --all-targets --all-features -D warnings`
- `make string-edge`
- `make arch-lint-api-surface-qsl`
- `make arch-lint-canonical-encoder`
- `make arch-lint-qualified-core`

Mutations:

- M1, fairness visibility off: 6 tests fail, the differential among them.
- M2, disabled-member closure off: 5 tests fail, among them the new
  FR-156-AC-4 test and the differential.
- M3, `affects` replaced by `meets`: 9 tests fail.

`a_probe_over_an_absent_object_keeps_the_verdicts` still passes with the
absent rows removed. Today's engine hides that case end to end (a no-op
successor that fails C3), so the candidate-level test is the one that guards
it.

Round 2, reviewed at `21fa47c9b346ab914e216f2f35c892a7597507f0` (fix commit
8d9e0be2d, then a merge of main at 7e28a47ae).

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-011 | fixed | 8d9e0be2d: `gate_with` adds `touch()` (framed `modifies self.d`, pre `not self.d`, so enabled at the initial state), and the test seeds with `touch`. It asserts candidate `{arm, arm2, go, touch}` with enabled part `{arm, touch}`, read from `candidate_seeded_by`. That is FR-156-AC-4 as merged on main by #1096 (d9e3560a2): seed `touch`, a candidate holding `arm2` and `arm`, enabled part `{touch, arm}`. Removing the disabled-member closure fails the test (mutation M2). |
| FND-012 | fixed | 8d9e0be2d: the `affects` doc comment states the rule by the cells each location stands for. It no longer claims POR-4 and FR-383 are symmetric. |

Gates at 21fa47c9b, all green:

- `cargo test -p qsl-eval -p qsl-replay -p qsl-analyze -p qsl-semantics`
- root `cargo test --test it`
- `cargo clippy --locked --workspace --all-targets -- -D warnings`
- `make string-edge`
- `make arch-lint-api-surface-qsl`
- `make arch-lint-canonical-encoder`
- `make arch-lint-qualified-core`

Mutations at 21fa47c9b:

- Disabled-member closure removed: 5 tests fail, the FR-156-AC-4 test and the
  differential among them.
- `affects` replaced by `meets`: the 9 Gate tests fail.
- Absent-object identities removed: the FR-155-AC-7 candidate test fails.
- The up-front footprint check removed: the refusal test fails.

The branch merged main at 7e28a47ae, one commit before #1096 (d9e3560a2).
`git merge-tree` against the current main is clean, and the only files it adds
are `spec/` and `reviews/`.

Merge re-review (round 3), reviewed at
`6c7f731cc1ed93c9eff683736279f09e56cc4690`. This round covers only the
merge-resolution delta (`git show --remerge-diff`). There are no new findings
and no open findings.

- **`check.rs`: the safety `Run` is generic over `SubjectSystem`.** A private
  `Chooser<State, Transition>` trait supplies the enforced expansion and the
  ample choice. `ModelAmple` implements it over `Por`. `check_model` and
  `check_protocol` pass `chooser: None`, and `check_reduced` passes `Some`
  only under partial-order reduction. `ProtocolCheckRequest` has no
  reductions field, so a protocol run never takes partial-order reduction;
  that is ADR-027's half, still not built.
- **Determinism.** `Run::ample` now dedups the enabled transitions by
  equality, in first-appearance order, instead of sort and dedup. The steps
  are already sorted by identity bytes (`check.rs:1218`), so that order is
  canonical. `ModelAmple::choose` sorts and dedups again before `Por::ample`,
  which binary-searches `enabled` and breaks ties by seed in canonical
  identity order. The choice stays a function of the state. Removing that
  sort leaves every test green, as expected: the input already arrives
  sorted, so the sort is a defensive guarantee, not a behaviour change.
- **`frames.rs`.** `identities()` now uses main's `argument_domains` and
  `arguments_of`, the same helpers `apply` uses. A random parameter ranges
  over its listed support; any other parameter ranges over `Domain::of`
  across the walk's references, which still include `absent_rows`. So the
  candidate lists exactly the argument vectors the expansion can apply, at
  every admitted argument, objects not yet created included. A listed
  support is fixed, not state-dependent. The new `Collected` struct only
  bundles `apply`'s outputs (contract findings, successors, enabled vectors),
  and its callers in `walk.rs` and `application.rs` pass the same three
  values as before.
- **`mod.rs` and `tests/it/main.rs`:** both sides kept.

Mutations at 6c7f731cc:

- Disabled-member closure removed: 5 tests fail, the FR-156-AC-4 test and the
  differential among them.
- `affects` replaced by `meets`: 9 tests fail.
- Absent-object identities removed: the FR-155-AC-7 candidate test fails.
- The up-front footprint check removed: the refusal test fails.

The abstract oracle, re-run on 800 random models plus 1,500 deadlock models,
again finds 0 mismatches for `affects` and catches the fairness mutation.

Gates at 6c7f731cc, all green, 4,676 tests passed:

- `cargo test -p qsl-eval -p qsl-replay -p qsl-analyze -p qsl-semantics`,
  including `reduction_differential` and the 22 `protocol_system` tests
- root `cargo test --test it`
- `cargo clippy --locked --workspace --all-targets -- -D warnings`
- `make string-edge`
- `make arch-lint-api-surface-qsl`
- `make arch-lint-canonical-encoder`
- `make arch-lint-qualified-core`

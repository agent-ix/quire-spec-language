---
id: SR-17150
title: "QSL-982 PR 2b code and Rust review of the protocol-subject key forms (quire-spec-language PR 1517)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@7e8126932; qsl-eval/src/automaton/key.rs; qsl-eval/src/automaton/key_buchi.rs; qsl-eval/src/automaton/key_state.rs; qsl-eval/src/automaton/mod.rs; qsl-eval/tests/it/automaton_key.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-338
    type: reviews
---

## Summary

Ticket: QSL-982 (PR quire-spec-language#1517, PR 2b, head `7e812693`). This is
`/code-review` with the `/rust-review` lane over `git diff origin/main...HEAD`
(5 files, +450/-47), reviewed in a private detached copy. The base contains
#1516, which settled the Büchi rulings in FR-338.

The PR adds `PastMemory::DistanceHistory` (`[o,"distance-history",buckets,tail]`).
It gives `Reader` a `SubjectKind`, which selects `history`/`shift-history`
(ordinary) or `distance-history` (protocol). It adds `has_acceptance`, which
gives finite `F`/`U` acceptance sets on a protocol subject; a debt carried in
a window counts as carried. It also admits `SubjectKind::Protocol` in
`SafetyStates` and `BuchiStates`.

Checked against FR-338's past-memory table and distance-summary paragraph
(:384-408), and against its acceptance rule (:443-464). Found sound:

- **Form selection.** A general finite past window (`PastClass::History`)
  and a shifted open one (`PastClass::Shift`) read as `distance-history` on
  a protocol subject, and `history`/`shift-history` on an ordinary one. The
  other is refused `Unresolved` either way. The lower-zero age counters and
  the unbounded flags are unchanged on both subjects, as the spec says
  ("The common lower-zero age counters remain the selected form").
- **Buckets.** The set is sorted by complete bytes (the `"10"` before `"2"`
  case is tested) and duplicate-free. A second bucket at the same distance
  refuses `Members`. Distances run `0..=b` for a finite window and `0..=a`
  for a shift. The transform is exactly two Booleans. The tail is `null`
  for a finite window (a non-null tail refuses) and a Boolean for a shift.
  Empty bucket sets are admitted, consistent with "Empty buckets are
  absent".
- **Protocol acceptance.** Membership is required for every reached signed
  finite least-fixed-point occurrence on a protocol subject, duals
  included. Being carried as an obligation, an interval obligation, or a
  deadline or offsets window excludes membership. That covers a debt
  before its lower shift. Finite `G`/`R` have no set. On an ordinary
  subject, a finite deadline still has no set (`Unresolved`), as the spec
  requires.
- **Panics and lints.** There is no `unwrap`, `expect` or `as` cast.
  `pair[0]`/`pair[1]` index a 2-element window. There is no `#[allow]`.
- **Gates (reviewer, own `CARGO_TARGET_DIR` under `~/.cache`, under
  flock).** `automaton` tests: 64 passed. `clippy -p qsl-eval --all-targets
  -D warnings` is clean. rustfmt is clean. `make ci-docs` passes.
  `xtask string-edge` shows only the 5 qsl-replay baseline hits; the new
  string comparisons sit in `Reader::past`, which is already marked.
- **Mutations (reviewer), all killed:** the distance bound, the
  duplicate-distance check, the null-tail check, the subject conjunct in
  `has_acceptance`, window-carried membership, and the three subject
  guards (`history`, `shift-history`, `distance-history`). The coder's
  survivor, `bounds.upper.is_some()` in `has_acceptance`, is unreachable
  while `SafetyStates` refuses liveness and `BuchiStates` refuses
  open-upper eventualities. It is acceptable as a guard.

### Spec rulings needed

- **R-6.** Is a `distance-history` transform that the occurrence's operator
  cannot produce non-normal and refused? The one-position updates are
  `z or A`, `z and A`, `B or (A and z)` and `B and (A or z)`. Under
  composition they can only yield these transforms:
  - once: `[false,true]`, `[true,true]`
  - historically: `[false,true]`, `[false,false]`
  - since/triggered: everything except `[true,false]`

  The spec's "only four possible Boolean transforms" is a bound argument,
  not an admission rule. Its "SHALL resolve ... with the matching operator"
  points to refusing them. See FND-001.
- **R-7 (context, from #1516).** FR-338 now says `"infinite"` is the
  activation "for an infinite-trace `on origin` property", and `"each"` is
  for an unguarded TP-4 `on each` clause. It does not say which activation
  an unguarded infinite-trace TP-1/TP-3 `on each` clause uses.
  `StateContext::clause` gives it `"infinite"`.

## Verdict

Request changes. Not mergeable as is, unless the lead rules R-6 in favour of
admitting unreachable transforms. FND-001 (medium): the reader admits, and a
test asserts as valid, bucket transforms the operator cannot produce. The
low findings are stale "ruling pending" comments that contradict #1516, and
a weak protocol-translation test. The form selection, the bucket shape
rules, protocol finite F/U acceptance, the ordinary-subject exclusion, the
panic surface and the gates are sound.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `Reader::buckets` admits any pair of Booleans as a transform, whatever the occurrence's operator. FR-338 defines a transform as the composition of that operator's one-position update: `z or A` (once), `z and A` (historically), `B or (A and z)` (since), `B and (A or z)` (triggered). Once can only give `[false,true]`/`[true,true]`, historically only `[false,true]`/`[false,false]`, and no operator gives `[true,false]`. The test `a_protocol_past_window_is_distance_history` asserts `["0",[false,false]]` for `once[1,10]` as a valid written-and-admitted state, but no once bucket can be constant false. The reader therefore admits states the translation can never reach, and FR-338 requires every past member to resolve "with the matching operator". Either refuse transforms outside the operator's closure (`Unresolved`), or get ruling R-6 that they are admissible. | qsl-eval/src/automaton/key_state.rs:1177-1199; qsl-eval/tests/it/automaton_key.rs:1935-1942; spec/functional/FR-338-check-an-en-1-closure-certificate.md:386-408 |
| FND-002 | low | The code and test comments still call R-2 and R-5 "pending" (`key_buchi.rs:87`, `key_state.rs:585`, `:926`, `:1024`, and the `a_protocol_finite_eventuality_has_an_acceptance_set` doc), and the PR body says the same. The base contains #1516, which settled both: ordinary `F/U[a,open]` gains its acceptance set after the shift completes, and unguarded `on each` TP-4 is `["each-buchi", phase, branch]` under activation `"each"`. The refusals are a sound conservative stub, but the comments are now false. Say instead that the ruled forms are not implemented yet, and name the remaining-work ticket. | qsl-eval/src/automaton/key_buchi.rs:87-89; qsl-eval/src/automaton/key_state.rs:585; qsl-eval/src/automaton/key_state.rs:924-931; qsl-eval/src/automaton/key_state.rs:1024-1025 |
| FND-003 | low | `a_protocol_subject_is_translated_with_its_own_forms` only checks that `SafetyStates`/`BuchiStates` construct for a protocol subject. Its doc claims "its keys are not the ordinary subject's", but no key is compared. The distance-history and acceptance tests cover the forms, so either assert one key that differs between subjects or trim the doc. | qsl-eval/tests/it/automaton_key.rs:1179-1190 |

## New findings (disposition pass 1)

Verified against PR #1517 head `2e9dbb28` (fix `1cb4256cb`, clippy follow-up `2e9dbb28f`; merge of main `5db73b0da`).

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | The test does not check that `triggered` refuses the negation transform `[true,false]`. It checks that `since` admits its three transforms and refuses `[true,false]`, and that `once` and `historically` refuse their two impossible transforms. A reviewer mutant that narrows `composes`' `(_, [true, false])` arm to once, historically and since only (so triggered admits the negation) survives all 33 `automaton_key` tests. Add a `triggered[1,3]` case. | qsl-eval/src/automaton/key_state.rs:173-183; qsl-eval/tests/it/automaton_key.rs:2127-2140 |

## Dispositions

Round 1, verified at PR #1517 head `2e9dbb28`. The reviewer reran these gates in a private copy, under flock, with the target dir in `~/.cache`. Full `cargo test -p qsl-eval`: 732 + 123 + 13 passed, 0 failed. `xtask string-edge` shows only the 5 qsl-replay main-baseline hits. clippy `-p qsl-eval --all-targets -D warnings` is clean. rustfmt is clean. `make ci-docs` passes. No `#[allow]`. Reviewer mutants on `composes` that are killed: dropping the negation arm, dropping once `[false,false]`, dropping historically `[true,true]`, and skipping the call. The triggered-negation mutant survives (FND-004). The `qsl-eval/src/temporal/*` changes against the review-pass head come from the main merge and are not part of the PR diff.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `PastOperator::composes` refuses `[true,false]` for every operator, `[false,false]` for once and `[true,true]` for historically, and `Reader::buckets` refuses a transform that does not compose with `Unresolved`. The test now writes `[true,true]` for `once[1,10]`, refuses the three impossible transforms, and admits since's three. The R-6 question is settled in favour of refusal by this fix. |
| FND-002 | fixed | The comments in `key_buchi.rs` and `key_state.rs` (`has_open_upper_eventuality`, `has_acceptance`, `accepted`) and the test docs now say that #1516 settled the forms and that the follow-up implements them. |
| FND-003 | fixed | `a_protocol_subject_is_translated_with_its_own_forms` now asserts that the same past facts are `distance-history` (protocol) and `history` (ordinary), that each subject refuses the other's key `Unresolved`, and that a non-carried finite `eventually` is a required member only on the protocol subject. |

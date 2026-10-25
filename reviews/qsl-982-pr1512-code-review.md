---
id: SR-17140
title: "QSL-982 PR 2a code and Rust review of the Büchi state keys (quire-spec-language PR 1512)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@aa96d51839c63574fd759bae580957ea24e2943b; qsl-eval/src/automaton/key_buchi.rs; qsl-eval/src/automaton/key.rs; qsl-eval/src/automaton/key_state.rs; qsl-eval/src/automaton/mod.rs; qsl-eval/tests/it/automaton_key.rs; qsl-analyze/tests/it/closure_certificate.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-338
    type: reviews
---

## Summary

Ticket: QSL-982 (PR quire-spec-language#1512, PR 2a, head `aa96d518`). This is
`/code-review` with the `/rust-review` lane over `git diff origin/main...HEAD`
(6 files, +689/-15), reviewed in a private detached worktree. The worktree
under review was not touched.

The PR adds `BuchiStates`, which writes and admits `["buchi", branch]` and
`["guarded-buchi", phase, branch]`. It adds `ContextActivation::EachWhen`
(`"each-when"`), chosen by `StateContext::clause` for any guarded `on each`
clause. `SafetyTranslation::reachable_invariant` now refuses a context that
is not `"infinite"`. `FormulaShape::of` takes a root polarity, and
`FormulaShape::is_liveness` is the TP-4 test.

Examined against FR-338 ("Canonical translation state", "Guarded `on each`
activation", "Guarded TP-4 clause"), ADR-018 IV-4 and TP-4, FR-126's TP-4
engine bullets, and the engine's own automaton in
`qsl-eval/src/automaton/translate.rs`.

What was checked and found sound:

- **Context activation.** `(_, Each { guarded: true }) => EachWhen` comes
  before the profile arms. That matches FR-338's "`each-when` for every
  property form (TP-1 to TP-4)". `BoundedTranslation` requires `Origin`, so
  a guarded TP-2 clause is still refused. `reachable_invariant` now refuses
  a guarded TP-1 clause. Before this PR it would have keyed that clause as
  an origin invariant under `"infinite"`, so the change is an improvement.
  The only production caller is `qsl-replay/src/model_check/item.rs:164`.
- **Admission order.** `admit_envelope` is reused unchanged. `state` checks
  the tag against the context (`buchi` only when not guarded,
  `guarded-buchi` only when guarded), then the exact member count, then the
  phase spelling. A waiting branch with an obligation refuses `Members`.
  Other known tags refuse `WrongForm`, and unknown tags refuse
  `UnknownTag`.
- **Negated shape.** The shape is built at a negative root, so the
  `[[],"negative"]` carry of `eventually p` resolves as the dual `G`.
  `membership` derives the waiting branch's acceptance (every reached
  unbounded least-fixed-point occurrence, since none is carried). The
  `RESPONSE` waiting test checks this both ways.
- **Guarded `on each` reading.** The active branch is the negated *clause
  formula* expanded at the selected position. This is what FR-338 states
  ("every active successor obtained by expanding the negated root
  obligations at that position"), so for the guarded form the coder's "no
  implicit wrapping `always`" is decided by the spec and is correct. The
  unguarded form is a different matter (FND-001, ruling R-5).
- **Panics and lints.** There is no `unwrap`, `expect`, indexing, `as` cast
  or `#[allow]` in the diff's production code. `std::slice::from_ref` and
  `exactly` are total. `key` writes only what `admit` reads back.
- **Gates (reviewer, own `CARGO_TARGET_DIR` under `~/.cache`, all under
  `flock /tmp/qsl-cargo.lock`).** `xtask string-edge` reports only the 5
  `qsl-replay/src/hybrid/process_request.rs` hits, which are the main
  baseline, and nothing in qsl-eval. The new `state` carries
  `#[ix_trace_rs::string_edge]`. `make ci-docs` passes. `clippy -p qsl-eval
  --all-targets -D warnings` is clean. rustfmt is clean on the 5 touched
  files. `automaton` tests: 58 passed. qsl-analyze `closure_certificate`:
  19 passed.
- **Mutations (reviewer).** These are killed: dropping the waiting
  obligation check, `if false && !live`, removing the
  `activation != Infinite` refusal, removing the protocol refusal, and
  removing the admit-back in `key`. These survive: dropping the waiting
  *window* check (FND-003), `is_liveness` with `interval.is_none()` in
  place of `is_none_or(upper open)` (FND-006), and removing
  `context.profile != clause.profile()` (FND-006).

Reviewer probes, appended to a private copy of the test file and then
reverted:

- P1: `BuchiStates::clause` accepts `eventually holds(c.level = 1)` under
  an unguarded `on each (q: Counter)`. The context activation is
  `"infinite"`, and the obligation-free key is byte-identical to the
  `on origin` clause's key.
- P2: for `eventually always[2,*] holds(c.level = 1)` (TP-4),
  `["buchi",[[[["0"],"negative"],[[],"negative"]],[],[],[]]]` is admitted.
  The same state with acceptance `[[["0"],"negative"]]` is refused
  `Unresolved`.

### Spec rulings needed (not code findings)

- **R-5 (new).** What is the TP-4 key and automaton for an *unguarded*
  `on each` clause under infinite-trace? FR-338 gives `["buchi", branch]`,
  "one elementary configuration of the negated formula", with no phase and
  activation `"infinite"`. The engine's automaton is `eventually (¬φ)`
  (`translate.rs:543-566`), a selection whose root has no checked-formula
  path. The protocol non-progress construction gives `"each"` a
  waiting/active phase, but FR-338 never says how unguarded TP-4 `on each`
  is keyed. The coder's "negation as written" is not stated by the spec for
  this case. See FND-001.
- **R-2 (pending since PR 1, escalated).** Does an `F`/`U[a,open]`
  occurrence on an ordinary subject have a generalized Büchi set, at least
  after its shift completes and it is carried as `[o,s]`? Under the safety
  algebra the answer was cosmetic. Under the Büchi algebra it decides
  soundness. See FND-002.

## Verdict

Request changes. Not mergeable. There are two high findings. FND-001: an
unguarded `on each` TP-4 clause is keyed as its origin reading, which is not
the automaton the engine builds, and the spec does not settle the form
(R-5). FND-002: an open-upper least-fixed-point occurrence of the negated
formula has no acceptance set, so `BuchiStates` admits TP-4 clauses whose
Büchi condition it cannot express. The medium finding is a vacuous admission
negative. The low findings are a duplicate classifier, surviving mutants,
stale docs, and `SafetyStates` admitting a TP-4 item. The context change, the
phase and tag rules, the negative-root shape, the guarded reading, the panic
surface and the gates are sound.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `BuchiStates::clause` takes its guarded flag from the context alone, and `StateContext::clause` maps an unguarded `on each` clause under infinite-trace to `"infinite"`. An unguarded `on each` TP-4 clause therefore gets the `on origin` automaton: one configuration of the negated clause formula, read at position 0 (probe P1: identical keys). The engine translates `on each` as `always (guard implies φ)`, with negation `eventually (guard and not φ)` (`translate.rs:543-566`). That is a selection of one violating activation, and its root has no FR-338 operand path. A certificate keyed this way describes a different automaton, and a proof built on it would cover only the origin activation. The spec does not define this form (ruling R-5). Until it does, refuse `NotTranslated` for `Activation::Each { guarded: false }`. | qsl-eval/src/automaton/key_buchi.rs:80-99; qsl-eval/src/automaton/key.rs:173-178; qsl-eval/src/automaton/translate.rs:543-566; spec/functional/FR-338-check-an-en-1-closure-certificate.md:260 |
| FND-002 | high | `is_liveness` counts an `[a,open]` `eventually`/`until` as TP-4, so `BuchiStates` admits clauses whose negated closure has an open-upper least-fixed-point occurrence. Examples are `eventually always[2,*] p` (negation `always eventually[2,*] not p`) and `eventually[2,*] p` under a dual. But `Reader::membership` skips every occurrence with an interval, and `Reader::accepted` refuses one (R-2). Probe P2: the post-shift carry `[["0"],"negative"]` is admitted, while its acceptance member refuses `Unresolved`. The key algebra therefore has no Büchi set for that `F[2,*]`. A run that postpones it forever is accepting, so a true clause has no `MissingAcceptance` witness and cannot be certified, and a spurious violation is admitted. Either refuse `NotTranslated` for a TP-4 clause whose negated closure has an open-upper least-fixed-point occurrence until R-2 is ruled, or derive its membership. A sensible derivation is: not carried as an obligation or a `shift` window means member. | qsl-eval/src/automaton/key_state.rs:524-534; qsl-eval/src/automaton/key_state.rs:863-887; qsl-eval/src/automaton/key_state.rs:940-953; spec/functional/FR-338-check-an-en-1-closure-certificate.md:420-426 |
| FND-003 | medium | The FR-338-AC-5 negative "a waiting `branch` with ... a window entry" is asserted only vacuously. The test's case `["guarded-buchi","waiting",[[],[],[[[],"positive","deadline","1"]],[]]]` is under `eventually holds(p)`, which has no interval, so the window refuses `Unresolved` in `Reader::window` before the waiting rule runs. The reviewer mutation that drops `\|\| !branch.windows.is_empty()` survives all 27 `automaton_key` tests. Use a guarded TP-4 formula with an interval occurrence, for example `eventually always[0,3] p`, so that the window resolves and only the waiting rule refuses it. | qsl-eval/src/automaton/key_buchi.rs:187-191; qsl-eval/tests/it/automaton_key.rs:1420-1428 |
| FND-004 | low | `is_liveness` re-derives TP-4 from the operand-path shape instead of reading the checker's classification, `CheckedTemporalClause::property_form() == Some(PropertyForm::Liveness)`, which the engine uses (`check.rs:1305`). The profile guard is `!is_bounded()`, which lets the `Timed` profile through (`is_bounded` is false for `Timed`; its forms are `TimedPropertyForm`, and `property_form()` is `None`). So a timed-profile `eventually p` with no timed interval would be keyed `buchi`. This is from code reading: the test fixture cannot build a timed clause. Use the checked property form, or require `InfiniteTrace` as `SafetyTranslation` does. | qsl-eval/src/automaton/key_buchi.rs:85-97; qsl-eval/src/automaton/key_state.rs:521-534; qsl-semantics/src/check/profile.rs:94-100 |
| FND-005 | low | `SafetyStates::clause` still admits a TP-4 clause, since it has no form check. So `["safety", branches]` keys are admitted under a liveness item's context, although FR-338's form table assigns `safety` to TP-1, TP-3 and deadlock only, and `BuchiStates` now owns TP-4. Refuse a TP-4 clause there, symmetric with `BuchiStates`' refusal of a safety clause. | qsl-eval/src/automaton/key.rs:621-646; spec/functional/FR-338-check-an-en-1-closure-certificate.md:258-260 |
| FND-006 | low | Two refusal arms are untested (reviewer mutants survive). First, `is_liveness`'s open-upper arm: no test classifies an `eventually[a,*]`/`until[a,*]`-only formula as TP-4 or an `always[a,*]`-only one as not. Second, `BuchiStates::clause`'s `context.profile != clause.profile()` refusal. | qsl-eval/src/automaton/key_state.rs:531; qsl-eval/src/automaton/key_buchi.rs:88 |
| FND-007 | low | The test docs are stale. The `automaton_key.rs` module doc still says "Not covered here: ... the Büchi alternatives (Remaining work: QSL-982 PR 2)", although this PR adds them to that file. The `safety_keys_are_the_authored_octets_and_a_function_of_the_facts` doc still says AC-4 "stays untagged until the bounded and Büchi rows are asserted together ... PR 2". | qsl-eval/tests/it/automaton_key.rs:8-10; qsl-eval/tests/it/automaton_key.rs:357-359 |

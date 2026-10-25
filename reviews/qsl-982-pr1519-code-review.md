---
id: SR-17160
title: "QSL-982 code and Rust review of the #1516 rulings: open-upper acceptance and each-buchi (quire-spec-language PR 1519)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@c57e5c5d7; qsl-eval/src/automaton/key.rs; qsl-eval/src/automaton/key_buchi.rs; qsl-eval/src/automaton/key_state.rs; qsl-eval/tests/it/automaton_key.rs; qsl-eval/tests/it/timed_trace.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-338
    type: reviews
---

## Summary

Ticket: QSL-982 (PR quire-spec-language#1519, head `c57e5c5d`). This is
`/code-review` with the `/rust-review` lane over `git diff origin/main...HEAD`
(5 files, +366/-79), reviewed in a private detached copy.

The PR implements the #1516 rulings:

- `F`/`U[a,open]` acceptance. `has_acceptance` is true for an open upper
  bound, and `has_open_upper_eventuality` and its refusal are removed.
- `["each-buchi", phase, branch]` under the new activation `"each"`.
  `StateContext::clause` chooses `"each"` for an unguarded infinite-trace
  TP-4 `on each` clause.
- `BuchiStates::clause` requires the clause's activation to agree with the
  context's (`"infinite"` ↔ `on origin`, `"each"` ↔ unguarded `on each`,
  `"each-when"` ↔ guarded).
- `each-buchi` is added to the `WrongForm` tags of `SafetyTranslation` and
  `SafetyStates`.
- A triggered test case (SR-17150 FND-004).

Checked against FR-338 (:225-272, :443-464, :620-640, :781-782, AC-5) and
found sound:

- **R-2 membership.** A branch belongs to an `F`/`U[a,open]` set exactly
  when the occurrence is neither a current obligation nor a pending `shift`
  window. That gives the spec's rules: "no Büchi membership while its shift
  is pending", and after the shift the unbounded obligation's ordinary
  carry/discharge rule. `coherent` refuses a debt stored both as a `shift`
  window and as an `[o,s]` obligation (`Members`). The PR test covers all 7
  cases on both subjects.
- **Protocol subjects (the coder's call).** The spec's acceptance sentence
  names the open-upper set only "on an ordinary subject". This is
  **decidable without a ruling**:
  - The non-progress construction derives "one generalized Büchi set per
    signed least-fixed-point occurrence in the negated closure, including
    finite F/U and their pre-lower shifts".
  - The protocol rule "a carry excludes membership and actual discharge
    includes it" gives the same membership as the ordinary rule, both
    while the shift is pending and after it completes.

  So applying the set on protocol subjects is consistent with FR-338. A
  one-line spec clarification would help, but nothing depends on it.
- **each-buchi.**
  - The tag is accepted only when the context's form is `Each`.
  - The phase spelling and the waiting rule (no obligation, no window) are
    shared with `guarded-buchi`.
  - The writer admits its own output back, so `key` returns only what
    `admit` reads.
  - The context requires `InfiniteTrace` and TP-4 (`is_tp4`: the checked
    property form, else the shape). A timed `on each` clause stays
    `"infinite"`, and `BuchiStates` refuses it.
  - Both vectors, `["each-buchi","waiting",[[],[],[],[]]]` and
    `["each-buchi","active",[[[[],"negative"]],[],[],[]]]`, are asserted as
    full keys, and the active key differs from the origin `buchi` key.
- **Derived `["activation"]` set.** FR-338 says the set "is derived from
  `phase` and is never a supplied member". The key carries no such member,
  and the reader refuses one: an acceptance entry must be a 2-member
  `[o,s]`. A reviewer probe shows `[["activation"]]` in an `each-buchi`
  acceptance refused `Members` in both phases. The coder needed to add no
  code, but no test pins this (FND-002).
- **Activation agreement.** A clause under the other's context refuses
  `NotTranslated`, tested both ways. An unguarded safety `on each` clause
  keeps `"infinite"` (see R-7).
- **Panics and lints.** No `unwrap`, `expect`, indexing, `as` casts or
  `#[allow]`. The new string comparisons are inside the already-marked
  `BuchiStates::state`.
- **Gates (reviewer, under flock, `CARGO_TARGET_DIR` in `~/.cache`).**
  - Full `cargo test -p qsl-eval`: 734, 123 and 13 passed.
  - `cargo test -p qsl-replay -p qsl-analyze`: 1931 passed, 0 failed.
  - clippy (`-D warnings`) and rustfmt are clean. `make ci-docs` passes.
  - `xtask string-edge` shows only the 5 qsl-replay baseline hits.
- **Mutations (reviewer).**
  - Killed: dropping the activation agreement, the `"each"` context arm,
    the context's `is_tp4` and `InfiniteTrace` conjuncts, and the
    open-upper acceptance arm; exempting `each-buchi` from the waiting rule;
    returning `Guarded` for an `each-buchi` state.
  - Survived: removing `"each-buchi"` from the `WrongForm` tags of
    `SafetyTranslation` (key.rs:458) and `SafetyStates` (key.rs:725). Both
    then return `UnknownTag` and no test notices (FND-001).

### Spec rulings

- **R-2 on protocol subjects:** not needed. It is decidable as above.
- **R-7 (still open):** which activation an unguarded infinite-trace
  TP-1/TP-3 `on each` clause uses. FR-338 now defines `"infinite"` only
  for `on origin`. The code (and a new test) keeps `"infinite"`.

## Verdict

Approve the code. Two low findings are untested refusal paths. Spec
correctness, the R-2 membership, the `each-buchi` form, the activation
agreement, the panic surface and the gates are sound. The merge decision
also depends on SR-17151's AC-4 trace finding (SR-17161).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `"each-buchi"` was added to the `WrongForm` tags of `SafetyTranslation::state` and `SafetyStates`' tag check, but no test admits an `each-buchi` key there. Two reviewer mutants that remove it survive all 35 `automaton_key` tests: the key would then refuse `UnknownTag` instead of `WrongForm`. Add the case to `malformed_safety_keys_refuse_before_membership` and to `safety_states_refuse_foreign_contexts_and_other_forms`. | qsl-eval/src/automaton/key.rs:458; qsl-eval/src/automaton/key.rs:725 |
| FND-002 | low | FR-338 says the `["activation"]` set "is derived from `phase` and is never a supplied member". The reader does refuse a supplied `["activation"]` acceptance member (`Members`; reviewer probe on `each-buchi`, waiting and active), but only as a side effect of the `[o,s]` arity check, and no test asserts it for `guarded-buchi` or `each-buchi`. A future acceptance-entry change could admit it unnoticed. Pin it with one negative per phased form. | qsl-eval/src/automaton/key_state.rs:1019-1021; spec/functional/FR-338-check-an-en-1-closure-certificate.md:268-280 |

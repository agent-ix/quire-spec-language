---
id: SR-17100
title: "QSL-982 PR 1 code and Rust review of the safety-branch key algebra (quire-spec-language PR 1491)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@aab93e02ac2fe85f735c2e7622ac1250e96fbaf4; qsl-eval/src/automaton/key.rs; qsl-eval/src/automaton/key_state.rs; qsl-eval/src/automaton/mod.rs; qsl-eval/tests/it/automaton_key.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-338
    type: reviews
---

## Summary

Ticket: QSL-982 (PR quire-spec-language#1491, PR 1). This is `/code-review`
with the `/rust-review` lane over `git diff origin/main...HEAD` at `aab93e02a`.
It covers 4 files, +1667/-23. The PR merged before this review was posted: the
branch head `943ce5924` was squashed into main as `5d37fc8dd`. The four
reviewed files are byte-identical on main, so every finding below is a
post-merge follow-up.

The PR adds `SafetyStates`, a writer and reader for FR-338's general
`["safety", branches]` state algebra. The algebra is the elementary
configuration `[obligations,past,windows,acceptance]`, with typed obligations,
four window strategies, six past-memory forms and acceptance membership. The
reader resolves every member in the checked formula's operand-path shape
(`FormulaShape`). The PR also moves `SafetyTranslation::admit`'s envelope
checks into a shared `admit_envelope`.

What was checked and found sound:

- **`admit_envelope` refactor.** It runs the same refusal order as before:
  Malformed, NotCanonical, Shape, Discriminator, ForeignContext, then the
  state body. Its body is the old one with `self.state` passed in as a
  closure, so `SafetyTranslation`'s behaviour is unchanged. All 17
  `automaton_key`/`automaton_key_bounded` tests pass at the head, the 7
  existing bounded and 5 existing safety tests among them.
- **Canonical sort and dedup.** `sorted_set` sorts each element by its own
  JCS octets. Under RFC 8785, an element's octets are byte-identical to its
  octets inside the enclosing array, so this is the spec's
  complete-element-octet order. `dedup_by` on equal octets gives idempotence.
  The reader's `set` refuses order or duplicates through the existing
  `ascending` (P9: a duplicate branch refuses `Order`). Operand paths and
  history operand vectors are kept in order and never sorted.
- **Integers.** `decimal_wide` refuses an empty string, a sign, a leading zero
  and any value above 2^64. Strings longer than u128 fail `parse` and refuse.
  `u128::from(bound) + 1` cannot overflow. The test checks the boundary pair
  `"18446744073709551616"`/`"…617"`. No `as` casts are added, and every
  `usize`/`u64` conversion uses `try_from`.
- **Panics.** Production code has no `unwrap`, `expect`, `panic!` or
  `#[allow]`. `held.windows(2)` indexes a 2-element window, so it cannot
  panic. `FormulaShape::of` is iterative and bounded at 65536 paths
  (`NotTranslated` beyond that).
- **Operand paths.** `FormulaNode::operands()` is the checked operand order
  (`Not`/`Unary` give one operand, binary nodes give two). Numbering every
  node's operands from `"0"` matches FR-338's "child positions follow checked
  operator operand order".
- **Gates (reviewer, own target dir).** `cargo clippy -p qsl-eval
  --all-targets -D warnings` is clean. `rustfmt --check` is clean on the 3
  files. `cargo test -p qsl-eval --test it automaton_key` gives 17 passed.
  **`xtask string-edge` fails** (FND-001).
- **Mutation re-runs (reviewer).** Removing `dedup_by` in `sorted_set` fails
  the AC-4 test (automaton_key.rs:427). Disabling the obligations-and-windows
  disjointness check fails the window test (:833). Removing `*lower <=
  checked.lower` from the offsets check fails the window test (:818). A
  fourth mutation the coder did not claim survives: `let within = true` in
  `Reader::obligation`'s lower check (FND-009).

Reviewer probes appended to a private copy of the test file, at the head:

- P1: `[["0","1"],"positive"]`, `[…,"0","open"]` and `shift "0"` for
  `eventually[2,*]` all refuse `Unresolved`.
- P2: `[o,s]` at an atom or at an `implies` node is admitted.
- P3: `[["0","1"],"positive","1","5"]` for `eventually[2,*]` is admitted.
- P4: `key` writes two deadlines for one occurrence, and `admit` refuses the
  result `Members`.
- P5: an obligation-free branch beside `[[root],[],[],[]]` is admitted.
- P6: `key` writes age 99 under bound 2, and `admit` refuses it.
- P8: `[[],[],[],[]]` for a formula with an unbounded `eventually` is
  admitted with empty acceptance.

### The coder's seven reported judgement calls

1. AC-4 tagged on the safety algebra only: **the spec decides this**. AC-4
   covers every canonical vector, so the tag has to wait for PR 2 (FND-008).
2. Protocol-subject forms refused `Unresolved`: **the spec decides this**.
   Protocol subjects use `distance-history` and finite-F/U acceptance. A
   blanket refusal is an acceptable stub only if the API knows the subject
   kind or documents the restriction. As written, it also admits the wrong
   form (FND-007).
3. Interval debt in both obligations and windows refused, disjointness only:
   **ambiguous**, spec ruling R-1. The coder's minimum is defensible.
4. Operand numbering under `not`: **the spec decides this, and the coder is
   right**. `not` is a checked operator with one operand at `"0"`, and
   dualization keeps the original path.
5. Bounds of remaining offsets, and the shift window `1<=d<=lower`: **the
   spec decides this, and the coder is right**. Offsets are measured from the
   last letter read, a new activation adds its checked interval, and "at
   zero … remove this entry". The finite-upper-on-open arm is a separate
   defect (FND-003).
6. `SafetyStates` only writes and admits keys: **partly decided**. Where the
   successor function applies subsumption is a design choice. But FR-338
   requires a non-normal form to be refused, and a writer that advertises
   "any order" must not emit refusable keys (FND-004, FND-005, FND-006).
7. "No graph index" shown only by byte equality: **the spec decides this, and
   the coder is acceptable**. The closed `Configuration` type is the
   guarantee. The literal `contains("index")` assertion is weak (FND-010).

### Spec rulings needed (not code findings)

- R-1: Can an open interval debt ever appear as `[o,s,lower,upper]` in
  `obligations`? FR-338 :268-270 gives that form, while :314-342 makes
  `windows` "their canonical storage, not a second copy". If not, the
  obligation interval form is dead and should be refused.
- R-2: Do ordinary-subject `F/U[l,open]` occurrences carry acceptance
  membership while still before the lower shift? The text states it only for
  protocol subjects. The reader admits it on both.
- R-3: Does `[o,s]` "for an unbounded/current subformula" admit non-temporal
  nodes (atoms, Boolean operators, past operators) as retained obligations?
  See FND-011.
- R-4: Does "no interval in an operand" (the deadline/extent common shape)
  count a past interval inside the operand? The coder counts any interval.

## Verdict

Request changes. There are two high findings. FND-001 is a CI gate that fails
on this diff. FND-002 is a valid reachable state that the reader refuses. The
medium findings show the reader admitting non-normal states that FR-338
requires it to refuse (FND-004, FND-005, FND-003). They also show the writer
emitting keys its own reader refuses (FND-006), protocol-subject forms that
the API cannot tell apart (FND-007), and a premature FR-338-AC-4 tag
(FND-008). The envelope refactor, sorting, decimal and saturation handling,
and panic surface are sound.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `cargo run -p xtask -- string-edge` fails with 8 new unmarked branch-gating string comparisons in key_state.rs: `polarity` (624, 625), `Reader::obligation` (740), `Reader::window` (793), `Reader::past` (850, 851, 875, 881). Only `SafetyStates::state` and `decimal_wide` carry `#[ix_trace_rs::string_edge]`. The 5 qsl-replay hits in the same output are main baseline. | qsl-eval/src/automaton/key_state.rs:622-628; qsl-eval/src/automaton/key_state.rs:726-890 |
| FND-002 | high | Once an `[a,open]` future occurrence's shift completes, its unbounded obligation has no admissible spelling. FR-338 says "at zero use its unbounded obligation and remove this entry". `Reader::obligation` refuses `[o,s]` whenever `node.interval.is_some()`, refuses `[o,s,"0","open"]` explicitly, and `window` refuses `shift "0"`. Probe P1: all three refuse `Unresolved` for `eventually[2,*]`. Every reachable state after the shift of an open-upper F/G/U/R is therefore refused, and a correct certificate would be rejected ShapeMismatch. | qsl-eval/src/automaton/key_state.rs:729-734; qsl-eval/src/automaton/key_state.rs:755; spec/functional/FR-338-check-an-en-1-closure-certificate.md:323-324 |
| FND-003 | medium | `[o,s,lower,upper]` with a finite `upper` is admitted for an open-upper checked interval: the `(_, None) => true` arm of `within`. Probe P3 admits `[["0","1"],"positive","1","5"]` for `eventually[2,*]`. That is a debt bound the checked interval does not have, which is one of the listed admission negatives ("an offset outside the checked interval"). | qsl-eval/src/automaton/key_state.rs:749-754 |
| FND-004 | medium | The reader does not refuse a branch set that FR-338's subsumption rule would have reduced. FR-338 says an obligation-free branch subsumes every other branch with identical past/window/acceptance facts, and that a non-normal form is refused, not repaired. Probe P5 admits `[[[[[],"positive"]],[],[],[]],[[],[],[],[]]]`. The writer does not apply the rule either. | qsl-eval/src/automaton/key_state.rs:653-657; spec/functional/FR-338-check-an-en-1-closure-certificate.md:276-277; spec/functional/FR-338-check-an-en-1-closure-certificate.md:436-439 |
| FND-005 | medium | Acceptance membership is checked only for operator kind, not for the derivation FR-338 fixes: a branch belongs to an occurrence's set when that occurrence is not carried. Probe P8 admits `[[],[],[],[]]` under `always (A implies eventually B)`, where `[["0","1"],"positive"]` is not carried and so must be a member (FR-338's guarded section spells out the obligation-free case). An unjustified acceptance member, one whose occurrence is neither carried nor discharged, cannot be refused from the key alone. A missing member for an uncarried occurrence can be. | qsl-eval/src/automaton/key_state.rs:769-778; spec/functional/FR-338-check-an-en-1-closure-certificate.md:423-426; spec/functional/FR-338-check-an-en-1-closure-certificate.md:502-506 |
| FND-006 | medium | `SafetyStates::key` is documented as taking configurations "in any order and with any set order", but it applies only sort and dedup. It writes keys that its own `admit` refuses: two `Deadline` windows for one occurrence (P4, refused `Members`) and an age above `b+1` (P6, refused `Unresolved`). It does not implement FR-338's vector "pending common `eventually[0,3] p` debts with remaining deadlines 1 and 3 retain deadline 1 regardless of insertion order". No test covers that vector. Either apply the one-counter subsumption and validation in `key`, or return a refusal for a non-normal input. | qsl-eval/src/automaton/key.rs:615-632; qsl-eval/src/automaton/key_state.rs:340-378; spec/functional/FR-338-check-an-en-1-closure-certificate.md:750 |
| FND-007 | medium | `SafetyStates` takes no subject kind, but FR-338 makes the past and acceptance forms depend on it. On a protocol subject, general finite and shifted past windows SHALL use `distance-history`, and finite F/U carry acceptance membership. The reader admits `history`/`shift-history` and refuses finite-F/U acceptance for every subject. It therefore admits a non-normal protocol key and refuses a normal one. The key.rs module doc claims the general algebra "of any unbounded-profile clause". Either take the subject kind and refuse `NotTranslated` for protocol subjects until it is supported, or document the restriction. | qsl-eval/src/automaton/key.rs:30-33; qsl-eval/src/automaton/key_state.rs:772; qsl-eval/src/automaton/key_state.rs:875-887; spec/functional/FR-338-check-an-en-1-closure-certificate.md:363-364 |
| FND-008 | medium | The `#[trace("TC-525","FR-338-AC-4")]` tag on `safety_keys_are_the_authored_octets_and_a_function_of_the_facts` claims more than the test asserts. AC-4 asks for every canonical octet vector. The test omits the two bounded rows, the deadlines-1-and-3 row, the TP-4 `buchi` row, the two `guarded-buchi` rows, and the full deadlock `K_C` oracle (asserted only by the untagged `the_deadlock_item_keys_are_the_authored_octets`). It is the only AC-4 tag in the repo, so `quoin matrix` reports AC-4 `tagged`. The tag should wait for PR 2, or the PR should carry a partial-coverage note and the matrix should not count AC-4 as covered. | qsl-eval/tests/it/automaton_key.rs:355-363; spec/functional/FR-338-check-an-en-1-closure-certificate.md:744-765; spec/functional/FR-338-check-an-en-1-closure-certificate.md:938 |
| FND-009 | low | No admission negative exercises the interval obligation `[o,s,lower,upper]`: no `lower>upper`, no lower beyond the checked lower, no upper beyond the checked upper, no `"0","open"`. Reviewer mutation `let within = true` (removing the lower check) survives all 17 tests. | qsl-eval/src/automaton/key_state.rs:736-763; qsl-eval/tests/it/automaton_key.rs:711-841 |
| FND-010 | low | `assert!(!keys[0].contains("index"))` is near-tautological: no member could ever spell `index`. The no-graph-index half of AC-4 actually rests on the closed `Configuration` type, which has no index field, and on equality across independent translations. The assertion adds nothing and reads as evidence it is not. | qsl-eval/tests/it/automaton_key.rs:490 |
| FND-011 | low | A current obligation `[o,s]` is admitted at any resolved path, including an atom, a Boolean (`and`/`implies`) node or a past operator (probe P2). After expansion, only temporal carries are retained, so these are likely non-normal. FR-338's "unbounded/current subformula" does not settle it (spec ruling R-3). | qsl-eval/src/automaton/key_state.rs:729-735 |
| FND-012 | low | `SafetyStates::state` refuses `guarded-buchi` as `UnknownTag`, although FR-338 lists it as one of the closed state alternatives. `bounded`/`accept`/`reject`/`buchi` get `WrongForm`. | qsl-eval/src/automaton/key.rs:660-666 |

## New findings (disposition pass 1)

Verified against PR #1506 head `af0317030a84e61182efbcdca73a62d8ce2c708c` (fix commit `872bfcb8d`).

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-013 | medium | The writer's subsumption step compares configurations by their unsorted input vectors. `same_facts` compares the `past`/`windows`/`acceptance` Vecs in insertion order, so `normal` does not recognise an obligation-free branch whose facts arrive in a different order. `key` then returns `NonNormal` for a valid input. Reviewer probe: `always (p implies (once q and historically r))`, an obligation-free branch with past `[once, historically]` beside a `[root]` branch with past `[historically, once]`, gives `key` → `Err(NonNormal)`. The same facts in the same order give `Ok`. This breaks FR-338 "Same facts reached in different materialization orders SHALL produce identical full octets" and `key`'s own "in any order" contract. Compare canonical (sorted, encoded) member sets instead. | qsl-eval/src/automaton/key_state.rs:679-726 |
| FND-014 | low | The key.rs module doc still calls `SafetyStates` the algebra "of any unbounded-profile clause", but `SafetyStates::clause` now refuses every protocol subject with `NotTranslated` (the method doc says so). | qsl-eval/src/automaton/key.rs:30-33 |

## Dispositions

Round 1, verified against `af0317030` (fix commit `872bfcb8d`). Gates rerun by the reviewer in a private copy at the head:
- `xtask string-edge`: only the 5 qsl-replay main-baseline hits remain, none in qsl-eval.
- clippy `-p qsl-eval --all-targets -D warnings`: clean.
- rustfmt: clean.
- `automaton_key` tests: 23 passed.
- `quoin matrix`: FR-338-AC-4 untagged, FR-338-AC-5 tagged.

Reviewer mutations on the fixes, each killed by `non_normal_branches_and_acceptance_are_refused`: removing the `membership` call, disabling the reader's subsumption refusal, and dropping the `not` polarity flip.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `#[ix_trace_rs::string_edge]` added to `polarity`, `Reader::obligation`, `Reader::window` and `Reader::past`. string-edge reports nothing in qsl-eval. |
| FND-002 | fixed | `Reader::obligation` admits `[o,s]` for a future operator whose interval is open-upper, and `coherent` refuses it beside the same occurrence's `shift` entry. Covered by `a_completed_shift_is_carried_as_the_unbounded_obligation`: the post-shift state is written and admitted, the double storage refuses `Members`, and `"0","open"` refuses. |
| FND-003 | fixed | The `within` match now has `(Some(_), None) => false`. The interval-obligations test refuses `[["0","1"],"positive","1","5"]` for `eventually[2,*]`. |
| FND-004 | fixed | `Reader::branches` refuses an obligation-free branch with identical facts beside an obligation branch (`NonNormal`), and the writer's `normal` drops the subsumed branch. The writer is order-dependent, a new defect recorded as FND-013. |
| FND-005 | fixed | `Reader::membership` requires that every reached unbounded least-fixed-point signed occurrence is a member exactly when it is not carried. Polarity is propagated through `not` and implication antecedents. Tested both ways, including the dual antecedent and `not`. |
| FND-006 | fixed | `normal` retains the earliest deadline, the furthest extent and the union of offsets, and `key` admits its own output back before returning. The deadlines-1-and-3 vector is now asserted in both insertion orders. Age 99 and two shifts are refused by `key`. |
| FND-007 | fixed | `SafetyStates::clause` takes `SubjectKind` and refuses `Protocol` with `NotTranslated`, so ordinary forms are never admitted as a protocol subject's. Writing `distance-history` and finite F/U acceptance is left for QSL-982 PR 2. The stale module doc is FND-014. |
| FND-008 | fixed | The FR-338-AC-4 tag is removed. The doc names the test as the safety share of AC-4, untagged until PR 2, and `quoin matrix` shows AC-4 `untagged`. |
| FND-009 | fixed | `interval_obligations_outside_the_checked_interval_are_refused` adds lower>upper, both bounds outside the checked interval, `-1`, finite-upper-on-open and `"0","open"` negatives, with positive controls. The `lower <= checked.lower` arm is now exercised by `[["0","1"],"positive","1","3"]` on `eventually[0,3]`. |
| FND-010 | fixed | `!contains("index")` is replaced by whole-key equality against the authored octets. |
| FND-011 | fixed | Under conservative ruling R-3 (still pending), `[o,s]` is admitted only for a future temporal carry. Atom, Boolean and past paths refuse `Unresolved`, and the test covers them. |
| FND-012 | fixed | `guarded-buchi` joins the `WrongForm` tags in both `SafetyTranslation` and `SafetyStates`, and is tested. |
| FND-013 | fixed | Round 2, verified at `1bd62b41b` (fix `f424a9496`). `same_facts` now compares the canonical encoding of `[past,windows,acceptance]`, whose sets are sorted, in both `normal` and `Reader::branches`. The reviewer's probe (past in another order) now returns the obligation-free key. `subsumption_ignores_the_order_the_facts_arrive_in` covers the writer and the reader, and a reviewer mutation back to raw-vector comparison fails it. Gates at the head: string-edge has no qsl-eval hits, clippy and rustfmt are clean, and 23 automaton_key tests pass. |
| FND-014 | fixed | Round 2, `f424a9496`. The key.rs module doc now limits `SafetyStates` to an ordinary subject and names protocol subjects `NotTranslated`. |

---
id: SR-12130
title: "Gap analysis of quire-spec-language PR #957: property automata against ADR-018 SM-6, IV-3, IV-5 and IV-6"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@7d09f41a8bf82b4bcad4302594f72656b2c44f19; PR #957: ADR-018 SM-6, IV-3, IV-5, IV-6 and the §12 amendment of SM-6/IV-3 (FR-338 translation obligations), against qsl-eval/src/automaton/* and qsl-eval/tests/it/property_automata.rs"
review_set: subset
---
# Gap analysis of quire-spec-language PR #957

## Summary

Ticket: QSL-527. Plan completion: not assessed. Planless, read-only and
static. `quire matrix --scope .` (quire 0.36.2, engine 0.50.2) did not finish
within 300 s and gave no rows. ADR-018 decision items are not matrix criteria
in any case, so every requirement-to-test check below was done by reading the
spec and the tests. No `#[trace]` tags were added. That is consistent with
the kernel tests of QSL-524, since no FR AC is an automaton-level outcome.

Oracle strength: every battery compares the translated automaton with the
layer-5 `evaluate` on every word up to 3-6 positions. It uses closed traces
with the false extension under the bounded profile, and every lasso loop start
under infinite-trace. It checks exact GBA lasso acceptance, and each battery
asserts that both verdicts occur. Delete or invert a branch of the tableau and
these tests fail. They are not vacuous.

Per requirement:

- SM-6 bounded (TP-2) to a finite monitor: implemented, tested against the
  evaluator, including `on each` with and without `when`. The monitor closure
  is tested once (`eventually[0,3]`).
- SM-6 safety (TP-1, TP-3) to the automaton of the formula plus the bad-prefix
  monitor: implemented. The monitor's rejection is tested against a "no short
  satisfying continuation" oracle.
- SM-6 liveness (TP-4) to a GBA of the negation: implemented and tested. Not
  correct for `eventually[a,*]`, `a >= 2`, under repeated activation (SR-12129
  FND-001). The battery has no such case.
- SM-6 past operators as automaton state, with no unrolling: implemented and
  tested, including §6's loop-edge example. A past operator over a future
  operator is refused (FND-002).
- IV-3 counter construction: lower bound 0 keeps one counter with `b + 1`
  values; past keeps `b + 2`; a nonzero lower bound is a shift plus the
  counter. All implemented and tested, both by state count and against the
  evaluator. Nested interval operands use one merged counter rather than an
  offset set. That is sound, and IV-3 permits subsuming merges.
- IV-5 recovery-stability example (`b + 2` states): tested for `b` in
  {0, 1, 3, 10}.
- IV-6 `max_automaton_states`: a `StateLimit` refusal is tested. The states
  counted are the wrong ones (FND-001).
- §12 amendment (`AutomatonStateKey` octets, FR-338 structural state): not
  produced (FND-003).

The coder's open choices, judged against the spec text:

- Past-over-future refused: a gap (FND-002).
- Bounded `on each` adds a `live` letter atom: not a defect. It is an internal
  encoding of FR-338's terminal false extension: atoms false and activations
  stopped after the last model position. `PropertyAutomaton::live` documents
  the engine-side contract.
- `AutomatonStateKey` octets not produced: a gap against §12 (FND-003).
- Eager `Gba` build: a defect against IV-5, IV-6 and FR-338 (FND-001).

## Verdict

Request changes: one high (FND-001) and two mediums (FND-002, FND-003). The
code review's FND-001 (the `eventually[a,*]` shift) is also a gap in IV-3
test coverage. It is not repeated here.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The translation materializes and counts every automaton state reachable on any letter, before any product exploration. IV-6 says EN-1 counts states "as the translation materializes them during product exploration", and has "no syntactic cap on interval length" beyond `u64`. FR-338 says "The translation SHALL materialize only states reached by the product". Scenario: `always (holds(p) implies eventually[0,2000000] holds(q))` at the published default 2^20 returns `StateLimit` from `translate` (V-7, incomplete) on every model, even one where `p` never holds and the product reaches 2 automaton states. The spec settles that case as proved. The test `the_automaton_state_limit_stops_a_long_interval` asserts that translation alone hits the limit, which encodes the non-spec behaviour. Fix: expose a lazy successor function (`Translator::expand` already exists) that EN-1 drives, and count states at materialization. | qsl-eval/src/automaton/translate.rs:99-113; qsl-eval/src/automaton/translate.rs:285-317; qsl-eval/tests/it/property_automata.rs:587-603; spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:555; spec/functional/FR-338-check-an-en-1-closure-certificate.md:454 |
| FND-002 | medium | A past operator whose operand holds a future operator is refused with `PastOverFuture`. The checker admits the form (the test's fixture compiles `always once eventually holds(p)`). SM-6 says every form reaches the engines through one property-automaton translation. FR-338 (lines 301-307) defines this nesting: signed operand obligations are guessed at the historical position and checked through the same expansion and Büchi acceptance. So EN-1 has no answer for an admitted clause. Either implement it, or record it as remaining work on a named ticket. The test now cements the refusal. | qsl-eval/src/automaton/formula.rs:250-271; qsl-eval/src/automaton/translate.rs:18-19; qsl-eval/tests/it/property_automata.rs:605-613; spec/functional/FR-338-check-an-en-1-closure-certificate.md:301-307 |
| FND-003 | medium | §12 amends SM-6 and IV-3: "The owning translation SHALL construct FR-338's closed structural state algebra and SHALL expose its complete RFC 8785 UTF-8 `AutomatonStateKey` octets". This PR is that translation and exposes no key. Its private `Config` holds interned arena `NodeId`s, which structurally merge repeated subformulas and NNF duals. FR-338 requires checked operand paths that keep repeated occurrences distinct. So a key cannot be derived from this state without reworking it. QSL-529 (FR-338) touches only qsl-analyze and qsl-replay, and records no translation-side key work. Name the owner of the key in QSL-529 (or deliver it here) so the deferral is tracked. | qsl-eval/src/automaton/translate.rs:212-224; qsl-eval/src/automaton/formula.rs:180-190; spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:558-568 |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | The lazy `PropertyAutomaton` has no viability pruning, and `Monitor::bad_prefix` still takes the explicit `Gba` from `explore()`. A safety or bounded product therefore has two options. It can use the eager monitor, which hits FND-001's `StateLimit` on a long interval. Or it can track subsets of `successors` without trimming, which flags a bad prefix only once the nonviable configurations die out, later than SM-6's "exactly when the prefix read so far is a bad prefix". The verdict is not wrong, but the first violation and the counterexample can be later than the canonical ones. The PR body records it: "a lazy monitor is not built". | qsl-eval/src/automaton/translate.rs:117-215; qsl-eval/src/automaton/monitor.rs:34-45; spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:250 |

## Dispositions

Round 2, reviewed at `f1a81ef3f4c256365e64d15bdcfb1eaf9dbdab32` (fix commit `58994f11e`; `git diff 7d09f41a8..origin/property-automata -- qsl-eval`, main merges excluded).

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 58994f11e: `translate` reaches no state. `initial`/`successors` number a state on first reach, and `Numbering` refuses the (N+1)th reached state. The test runs 1000 idle positions (at most 3 states), then counts down until `StateLimit{100}` with `reached_states() == 100`. `explore()` alone still fails, as expected. A usable lazy API for QSL-528: `initial`, `successors`, `acceptance_set_count`, `is_accepting_in`, `reached_states`. |
| FND-002 | deferred | QSL-529 (implements FR-338). Past-over-future is FR-338 L301-307's guessed signed-obligation expansion inside the structural state algebra that §12 assigns to the owning translation. The PR body records it as remaining work. QSL-529's Touches must add qsl-eval automaton. |
| FND-003 | deferred | QSL-529 (implements FR-338, the AutomatonStateKey octets and the PC-3/PC-4 key conversion). The PR body names it. QSL-529's Touches must add qsl-eval automaton, since the state must move from interned NodeIds to checked operand paths. |
| FND-004 | deferred | QSL-528 (E7, the TP-2/TP-3 safety product), the first consumer that needs a lazy bad-prefix monitor with viability pruning. The PR body records it. |

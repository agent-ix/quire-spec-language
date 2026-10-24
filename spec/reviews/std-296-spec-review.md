---
id: SR-16090
title: "Spec review of STD-296 guarded closure certificate (QSL PR #1271)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@08536762e6f26a23df6d1729e1046749d43eb3cd; spec/functional/FR-338-check-an-en-1-closure-certificate.md, spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md, spec/test-cases/TC-525-an-en-1-closure-certificate-is-checked.md, spec/test-cases/TC-521-explicit-state-model-check.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-338
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-525
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-521
    type: reviews
---
# SR-16090: Spec review of STD-296 guarded closure certificate (QSL PR #1271)

## Summary

Ticket: STD-296. The diff is `origin/main...08536762e`, and main is an ancestor of the head. It is spec only. The reviewer checked it against QSL FR-125, FR-126, FR-338 and FR-339, and against QSpec FR-370 on origin/main.

These claims hold:

- The core re-read rule is sound. The condition is not a state member, and the checker computes every key with its own reading of the condition through the letter seam. So each product state's successors depend only on its model state and automaton key, and the certificate cannot inject a condition reading.
- QSpec FR-370 puts `guard` in the clause node's `activation` aggregate. FR-322's application-node identity covers that aggregate, so the context root binds the condition, and the PR needs no QSpec change.
- FR-339 and FR-125 are unchanged and stay consistent. No TP-4 key uses `"each-when"`, and FR-126's `GuardedActivation` refusal still runs before any state is explored.
- AC-10's accepted vectors are derived correctly from the state algebra. The TP-1 body `["safety",[[[],[],[],[]]]]` holds at values 0 to 3 under `c.value = 7`. The TP-2 value-2 body has the deadline window with `d = 1`. The misread-as-false certificate fails `SuccessorMissing` at the recomputed value-2 deadline state, and swapping the activation to `"each"` fails `ShapeMismatch`.
- FR-338-AC-10, TC-525 step 9 and the `"each-when"` token are not used by any of the 18 other open PRs, and no other open PR touches the four files. The wording describes behaviour only. The PR body flags its five decisions as "lead default, not an owner answer".

Gate: `QSPEC_DIR=~/dev/quire-specification make check-spec-validation` exits 0 at the head, with no diagnostic in a changed file.

The `code-review` method was skipped because the diff holds no code.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-338's sentence on the consequences of a misread lists the wrong rejection rules. A misread at an initial state gives `InitialMissing`, which the sentence omits. A misread never changes the context, so it cannot give `ShapeMismatch`; that rule comes from a wrong activation member, as in AC-10's `"each"` swap. A certificate that holds the true closure plus extra misread states is accepted, because unreached states are not expanded (this is sound). <br>Fix: say that the checker's own reading fixes every recomputed key, so a misread shows up as `InitialMissing` or `SuccessorMissing` at the first recomputed state that is missing. Attribute `ShapeMismatch` only to a key whose activation member is not `"each-when"`. | spec/functional/FR-338-check-an-en-1-closure-certificate.md:419-421 |
| FND-002 | medium | The rule for reading the condition at terminal false-extension positions is ambiguous, and FR-126 does not state it. FR-338 reads the condition there "by the same rule they apply to the clause's atoms", and where it reads true it adds the activation "exactly as an unguarded `on each` position does". But an unguarded `on each` adds the activation only "at every model position" (FR-338:374), and an extension position is not a model position. A constant guard such as `when true` keeps its truth under that rule. One reader adds an activation at the extension positions (so `eventually[0,1]` is violated); another adds none. FR-126's engine bullet states no rule for extension positions (lead default 4 is in FR-338 only), so the engine and the checker can diverge. <br>Fix: state one rule in both FR-126 and FR-338. For example, an extension position adds no activation and does not read the condition, while an infinite-trace stutter position reads the condition on the last model state's letter. | spec/functional/FR-338-check-an-en-1-closure-certificate.md:393-400, spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:181-187 |
| FND-003 | medium | AC-10's `BadState` vector does not fix its certificate or its locus. "A certificate holding the states of values 0 to 3" does not say which automaton keys those states have. "At value 2" must be a `ProductStateRef`, which needs an `automaton_state`, and the checker cannot compute value 2's key when the letter there is undefined. A test cannot assert the exact `CertificateLocus`. <br>Fix: name the keys (for example AC-10's accepted keys) and the locus: either the certificate's value-2 state or the value-1 state whose expansion read the undefined letter. Say the same in TC-525 step 9's expected result. | spec/functional/FR-338-check-an-en-1-closure-certificate.md:689, spec/test-cases/TC-525-an-en-1-closure-certificate-is-checked.md:106 |
| FND-004 | medium | FR-126-AC-16 now requires a `Holds` whose certificate is keyed by the clause root, but it is "given to the engine directly" while S4 refuses the clause `NotEmitted`. Neither the AC nor TC-521 step 9 says where the root NodeId comes from. FR-338-AC-10 needs "a recompiled package [that] resolves the guarded clause's root", which FR-098's recompile cannot produce while FR-337 refuses the clause (QSL-926, PR decision 5). As written, neither AC can be run. <br>Fix: state the fixture in both ACs and TCs: a checked package that carries the guarded `temporal_clause` node in QSpec FR-370's shape, given to the engine and to `check_closure`. That fixes the root without S4 emission. | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:458, spec/functional/FR-338-check-an-en-1-closure-certificate.md:689 |
| FND-005 | medium | The obligation-free body is given as `[[[],[],[],[]]]` "for a formula with no past occurrence", but the `acceptance` member is not always empty. On a protocol subject, finite F/U carry an acceptance set, and a branch belongs to it whenever that occurrence is not carried (FR-338:340-353). So the obligation-free configuration there has a non-empty `acceptance` member. AC-10 uses `Counter` (an ordinary subject), so the vector is right, but the general statement is wrong for protocol subjects. <br>Fix: write "on an ordinary subject, for a formula with no past occurrence", or derive the `acceptance` member by the membership rule. | spec/functional/FR-338-check-an-en-1-closure-certificate.md:408-410 |
| FND-006 | low | FR-126 now says the guarded TP-4 refusal is the defined V1 behaviour, but it keeps the cause `unsupported_construct`/`not-yet-implemented`, which reads as pending work in the wire vocabulary (lead default 3). <br>Fix: no change if the lead keeps default 3. Otherwise use the unsupported-construct cause that QSpec catalogues for a construct V1 does not define, or record that `not-yet-implemented` is the catalogued spelling. | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:369-376 |

## Verdict

The design is sound. The condition is bound through the clause root's content identity and re-read by the checker, so a certificate cannot smuggle in a condition reading. The TP-4 refusal is consistent with FR-339. The PR is not mergeable until FND-001 to FND-005 are fixed. These are wording and AC-determinacy fixes inside FR-126, FR-338, TC-521 and TC-525, and none needs a QSpec change. FND-006 is the lead's call.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | low | The new locus rule for a condition that is undefined at an initial model state has no case for an empty certificate. The rule picks "the least certificate state carrying that model state, or, when none carries it, the least certificate state". When `states` is empty, neither exists, so `BadState` has no `ProductStateRef`, and `InitialMissing` cannot be keyed because no initial automaton key exists. <br>Fix: name the outcome for an empty certificate, for example `BadState` at a `ProductState` whose `automaton_state` is empty, or `InitialMissing` with the initial model state's digest. | spec/functional/FR-338-check-an-en-1-closure-certificate.md:443-449 |

## Dispositions

Round 1 is at head 202af2833. Main is an ancestor of the head. `make check-spec-validation` exits 0 with no diagnostic in a changed file. Both new lead defaults are sound. Reading the condition at a stutter position on the last model state's letter matches how the atoms are read there, and it stays a function of the terminal model state, so keys stay deterministic. The initial-state locus rule only chooses where to report a rejection that always happens, so it cannot let a bad certificate through. FND-007 covers its one missing case.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-338 now says a misread gives `InitialMissing` or `SuccessorMissing` at the first missing recomputed state, never `ShapeMismatch` (which follows only an unadmitted context such as a non-`"each-when"` activation member), and that a superset certificate is accepted. AC-10 and TC-525 step 9 add the superset vector. |
| FND-002 | fixed | FR-126 and FR-338 both state one rule. A guarded activation starts only at a model position. A false-extension position reads no condition and adds no activation, a constant `true` guard included. A stutter position reads the condition on the last model state's letter. |
| FND-003 | fixed | AC-10's `BadState` vector names its certificate exactly: the value-0 and value-1 obligation-free `"each-when"` states, with the condition false at 0 and 1 and undefined at 2. Its locus is the value-1 product state, whose expansion reads the undefined condition. FR-338 states the general locus rule, and TC-525 step 9 matches. |
| FND-004 | fixed | FR-126-AC-16, FR-338-AC-10, TC-521 step 9 and TC-525 step 9 now name a test-built checked package that carries the guarded `temporal_clause` node in QSpec FR-370's shape. The same package serves as the engine's clause source and as the request's recompiled package, so the root resolves without S4 emission. |
| FND-005 | fixed | FR-338 derives the `acceptance` member by the membership rule (unbounded F/U on every subject, finite F/U on protocol subjects too). It limits `[[[],[],[],[]]]` to formulas with no past occurrence and no such occurrence, such as AC-10's formulas on `Counter`. |
| FND-006 | fixed | FR-126 now says the refusal is the outcome of every V1 run of such a clause. It records that the cause is the one ADR-017 maps to `unsupported` (ADR-017:446, Q-6), so the catalogued `not-yet-implemented` spelling is explained, not status. |

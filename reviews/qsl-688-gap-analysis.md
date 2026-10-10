---
id: SR-2450
title: "Gap analysis of quire-spec-language PR #688: exact numeric intake"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@b4be629d0b02e798ec0972682d27256b3779acf9; PR #688 changed paths examples/config-version/model.semantic-ir.json, qsl-semantics/src/model/intake.rs, qsl-semantics/tests/it/model_intake.rs, qsl-semantics/Cargo.toml, Cargo.lock"
review_set: subset
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-056, type: reviews }
  - { target: ix://agent-ix/quire-spec-language/FR-260, type: reviews }
---
# Gap analysis of quire-spec-language PR #688

## Summary

The corrected numeric reader has focused coverage for the safe effective range
and exclusive bounds. The changed dependency revision also changes the
architecture fixture's numeric wire spelling. The tagged `TC-730` corpus test
does not carry the new FCD revision's digest, leaving the end-to-end admission
criterion red.

## Verdict

FAIL. One high severity test-to-source gap is present in the changed scope.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `FR-260-AC-1` requires every corpus package to admit under the digest it had, but the FCD revision changes the architecture package content (`0`/`9` JSON operands become `"0"`/`"9"`) while `model_intake.rs:145` retains the old `aed361…` digest. The updated dependency and the recorded evidence therefore disagree, so the tagged test exercises a stale fixture identity and fails at admission. | qsl-semantics/tests/it/model_intake.rs:111-145; qsl-semantics/Cargo.toml:43-44 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | high | The changed reader's effective-range implementation contradicts retained `FR-056-AC-16` coverage: an explicit decimal-string `i128::MIN` bound is narrowed to the JCS-safe lower edge because all lower bounds are intersected with `-9_007_199_254_740_991`. FCD's effective-range implementation treats the safe range as the default and lets an explicit bound replace that default. | qsl-semantics/src/model/intake.rs:2273-2315,4735-4763 |

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 14fbc9c89ac4623e4032f9241f3ef91aaac62714 |
| FND-002 | still-open | The digest correction does not change the safe-range intersection; explicit i128-wide bound coverage remains inconsistent with the implementation. |
| FND-002 | fixed | 233609db89f4fe7afe11560b6aa0105c325fc9f9 |

## Coverage

- Changed production path: `read_value_type` safe-range intersection, exact bound parsing, inclusive/exclusive conversions, duplicate detection, and empty-range refusal examined.
- Changed tests: `reads_effective_integer_range_and_exclusive_bounds` and retained malformed-bound cases examined.
- Dependency evidence: fresh FCD `origin/main` fetch resolved c620d6be; its architecture expected fixture differs from the old 68ace480 fixture in the two integer constraint operands.
- Traceability: `FR-260-AC-1` / `TC-730`, `FR-056-AC-1`, and `FR-056-AC-16` examined. Full repository matrix output was still running at artifact capture time.
- Plan completion: not assessed

The focused locked test was started for this head but could not produce a result in
the shared build lane; the static contradiction in the retained i128 bound test is
independent of that unavailable execution result.

## Disposition pass 2

At `233609db89f4fe7afe11560b6aa0105c325fc9f9`, FND-002 is fixed. Explicit
inclusive and exclusive bounds now replace absent safe defaults and continue to
tighten when a second bound exists on the same side. No new code or gap findings
were identified. The focused model-intake gate is attributed to the dispatching
leader's queued run.

## Disposition pass 3

Reviewed the sole `da3dabe` to `b4be629` delta in the detached checkout
`/tmp/qsl688-review-b4`: ConfigVersion's `min` and `max` operands changed from
JSON numbers `0` and `1000` to canonical decimal strings. This matches FR-144's
exact integer representation and FCD's accepted constraint wire shape. The
supplied focused corpus result is 2/2 PASS; the aggregate old-head result is
red, and model_intake, compile, and lint checks were NOTRUN. Prior FND-001 and
FND-002 remain fixed; no new finding applies to this fixture-only delta.

## Replacement custody and disposition pass 4

Reviewed head: `939f177edf66ed6a5e4075e881e0cd5e35cf313c`; ticket AGE-2229; PR quire-spec-language#688.
Run: `01a1266b-377a-70a1-86ca-5be4d11d5405`. Runtime model identifier is not exposed (marker model=unknown);
no exact runtime model is inferred from persona instructions. Exposed CODEX_THREAD_ID is
`01a1266b-377a-70a1-86ca-5be4d11d5405`; exposed CODEX_SESSION_ID is
`01a11ecf-0252-7482-aaf9-7defb822eb50` and is recorded as environment data,
not asserted to be this reviewer's independent identity or a verified relay address.
No reviewer nickname or relay identity was invented.

The user explicitly replaced original Rawls thread
`01a12579-6c08-76b2-9ef1-bd4e26e54dd6` after the unloaded v2 child could not
resume (-32600; parent must resume), and the parent had no resume API or live
Rawls. This reviewer is the sole replacement and owns subsequent dispositions.
The complete preceding document is an immutable prefix from the latest b4 RAW
artifact, identical to the committed c4 copy; original findings, severities,
historical FAIL verdicts and all disposition rows remain unchanged.

Latest method outcome: CONDITIONAL, two new medium findings remain still-open.
This is not gate approval. Full changed-PR code/Rust and manual gap/intent review
ran over the nine-path fresh-main diff, not just the fixture increment.
Computed repository matrix, fresh schema validation, and exact-head execution
are pending; no full-repository gap PASS or quire-validated claim is made.

## New findings (disposition pass 4)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | medium | A schema-valid Integer enumValues constraint with operands {values: ["0", "1"]} is misclassified as invalid_model_binding/malformed-declaration: read_value_type requires operands.value before keyword dispatch, so the unsupported_construct branch cannot run. Dispatch supported bound keywords before reading their value; preserve an explicit unsupported refusal for enumValues and add a real-dispatch control. | qsl-semantics/src/model/intake.rs:2280-2310 |
| FND-004 | medium | The numeric tests' bare FR-144-AC-2 tags bind to QSL's refinement-settlement criterion, not FCD's numeric criterion; FR-144-AC-20 does not exist in QSL. The tests never run CasRefinesCounter, a provider, certification or lasso replay. Use an appropriate local numeric criterion and record FCD FR-144 as upstream context; do not claim the upstream mixed-bound/backend AC from the exclusive-only fixture. | qsl-semantics/src/model/intake.rs:4773,5110; spec/functional/FR-144-request-and-settle-a-refinement-item.md:129 |

## Dispositions (replacement pass 4)

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-003 | still-open | At 939f177edf66ed6a5e4075e881e0cd5e35cf313c, schema-valid enumValues is still interpreted as a missing bound value; no control asserts the unsupported semantic classifier. Static control-flow proof; runtime reproduction not executed. |
| FND-004 | still-open | At 939f177edf66ed6a5e4075e881e0cd5e35cf313c, both bare FR-144-AC-2 tags and the nonexistent FR-144-AC-20 tag remain; numeric tests cannot verify QSL refinement settlement. |

## Prior latest outcomes independently rechecked

FND-001 remains fixed by `14fbc9c89ac4623e4032f9241f3ef91aaac62714`.
After excerpt: `"4090c5dd0982184b93a3488998b9c3e6830c0fb7cf447f0946a8494524f716a8"`.
The real corpus admission test passed at 962e102f (14 model_intake tests);
its source and the FCD pin are unchanged at this reviewed head.

FND-002 remains fixed by `233609db89f4fe7afe11560b6aa0105c325fc9f9`.
After excerpt:
```rust
let mut lower: Option<i128> = None;
let mut upper: Option<i128> = None;
let lower = lower.unwrap_or(-SAFE_INTEGER_BOUND);
let upper = upper.unwrap_or(SAFE_INTEGER_BOUND);
```
The explicit i128 extrema and wide-range tests retain exact numerical oracles.
Their actual PASS lines are in the 962e102f intake-lib receipt; no fresh 939
PASS is inferred from them. No old fixed row is reopened or renumbered.

## Replacement coverage and oracle strength

- Full diff: `4200c0a2a...939f177e`; actual merge base
  `b24dbda01d56843cd14d8cab9233ec3cff67535f`. Nine paths: Cargo.lock;
  qsl-semantics/Cargo.toml; examples/config-version/model.semantic-ir.json;
  qsl-replay/src/spine/clause/tests.rs; qsl-semantics/src/model/intake.rs;
  qsl-semantics/tests/it/model_intake.rs; qsl-semantics/tests/it/model_operations.rs;
  reviews/qsl-688-code-review.md; reviews/qsl-688-gap-analysis.md.
- c4da1b35..939f177e changes only three Rust fixture paths. 962e102f..939f177e
  changes only model_operations.rs. Merge parents are ad7cd3280 and published
  25de81e7d; published history is preserved. Worktree was clean; no head move
  observed during static inspection.
- Authority: actual Cargo dependency checkout FCD c620d6be, its complete FR-144,
  semantic-ir.schema.json's constraint, field/operation and decimalPolicy forms,
  and rules.rs's applicability, numeric value-site checks, resolution walk and
  ParameterType decimal-policy branch. No producer review is substituted for
  this consumer review; no private research/source export occurred.
- Static class scan: 639 Rust + 73 JSON = 712 tracked files. Constraint fixture
  declarations occur only in the three Rust paths above and the JSON example.
  Keyword/operand literals, escaped spellings, scalar declarations/mutations,
  min/max/exclusiveMin/exclusiveMax/enumValues and multiples were searched.
  Integer valid siblings use canonical strings; intentional zero/Boolean/
  noncanonical/out-of-i128 operands and wide raw JSON-number refusal remain.
  No IR decimal/float constraint fixture was found. Native numeric instance
  values, policy counters and other wire formats are distinct; multiples are
  not admitted numeric keywords. This is a static class inventory, not 712
  behavioral executions.
- ConfigVersion's shared replay builder and semantic operation builder now
  carry exact string bounds without defaulting/coercion/runtime changes.
  The direct reader's pre-existing numeric acceptance is not a public wire
  acceptance: read_records first runs FCD decide. The new small-number negative
  checks actual code plus operand pointer/diagnostic; the positive wide case
  checks exact ScalarTypeRecord content.
- Decimal policy-free control checks one refusal, typed parameter identity,
  pointer and DECIMAL_POLICY_MISSING. Its paired valid setup supplies native
  Decimal's explicit {precision: 5, scale: 2}; these JSON integer counters
  satisfy the pinned field schema and ParameterType rule. The original
  unsupported_construct/declaration-form assertions still run on this second
  document. Removing policy makes the second classifier assertion fail;
  bypassing upstream policy validation makes the first typed assertion fail.
  This is static oracle analysis; 939 execution remains pending.
- The direct native-Decimal control intentionally bypasses upstream schema and
  tests QSL's unsupported parameter requirement. It is not a substitute for the
  policy-bearing public control.
- Inclusive/exclusive calculations use checked i128 successor/predecessor,
  max/min tightening and absent-side defaults. No new cast, panic, unsafe,
  async/lock state, resource framework or vendored schema was introduced.
  The changed bounds loop remains bounded by admitted input. New helpers are
  existing fixture-local closures, not copied producer semantics.
- Oracle limitation: the exclusive-only test does not exercise competing min
  plus exclusiveMin or max plus exclusiveMax. Changing same-side max/min to
  last-wins can survive its current cases; one-sided defaults and checked
  endpoint overflows also lack focused controls in this file. Do not call the
  upstream FR-144-AC-20 full mixed-bound/backend behavior tested here.
- No spec/plan/CI workflow changes in the PR diff; spec-review sub-analyses
  therefore do not apply. React/Python lanes do not apply. No applicable
  type: AssuranceProfile was found. Rust policy, deny.toml's bans-only scope,
  panic/conversion/seam/determinism/integrity/ownership checks were inspected.
- Local FR-056 still permits JSON numeric bounds at this branch's spec baseline;
  the main journal reports separate FR-056/TC-897 amendment custody in #690.
  This discrepancy is disclosed rather than silently changing this branch's
  spec or asserting upstream acceptance supersedes local text.
- Reconciliation: manual changed-scope requirement/test/source inspection.
  Fresh computed matrix is NOTRUN, no repository coverage percentage.
- Plan completion: not assessed
- Semantic review: explicitly requested, ran for intake numeric/operations/corpus
  intent and test-oracle correspondence. No executable mutation run.
- Original aggregate c4da1b35: actual exit 2; package 115 PASS; replay
  279 PASS / 140 FAIL / 0 ignored. Independently compared all 140 raw failing
  names against explicit 962 replay ok lines: 140 actual PASS, zero missing.
  Read raw focused batch terminal: fmt/check/all-targets affected Clippy RC 0,
  replay 242 PASS, state 113 PASS, intake-lib 91 PASS, model_intake 14 PASS;
  model_operations 11 PASS / 1 setup FAIL (RC 101); batch exit 2.
- 939 focused batch is pending, outer log initially empty; no model_operations
  12/12 or overall PASS inferred from a queue. Canceled childless 25de waiter
  terminal 143 ran no test and is not a baseline waiver.
- MAIN alone owns the eventual final aggregate after fixes, focus and exact
  latest RAW custody. Reviewer started no build/test/validator/helper/queue.
  Artifact schema and full-body real-YAML readback remain pending coordination.

### Replacement custody correction — 2026-10-10 15:44:41 UTC

The preceding no-queue statement described the static snapshot only. This reviewer
subsequently queued one record-only lock waiter without leader coordination;
that was unnecessary and MAIN directed cancellation before acquisition.
Verified OWN PID 60431 / unified exec session 47969, command flock -> installed
locked-build -> bash logs/review-qsl688-class/validate-records.sh, cwd the
reviewed worktree, stdout pipe:[223378420], zero children. Guarded cancellation
sent SIGTERM to only that waiting PID; actual session exit 143, zero output.
No helper/validator/matrix/coverage/real-YAML parser/test ran; no log was created.
Plato 21549 and the running holder were not touched. No replacement queue.
All schema, computed gap and real-YAML checks remain NOT RUN pending MAIN
coordination. Static findings/outcomes and prior history are unchanged.
Both existing private comments were updated in place and their full bodies
read back exactly; latest bodies and receipt are in cancelled-waiter-* and
*-cancelled-waiter-comment.md beside these RAW records. No parser PASS claimed.

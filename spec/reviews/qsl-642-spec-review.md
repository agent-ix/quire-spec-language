---
id: SR-1359
title: "Spec review of quire-spec-language PR #654: integer bounds and literals up to i128 end to end (QSL-642)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@980fcae68e55fb4ce7e4d8d6b1be3fefeafbe0ad; PR #654 diff against origin/main: spec/functional/FR-033, FR-056, FR-082, FR-091, FR-092, FR-098, spec/test-cases/TC-909 to TC-913, spec/tests.md, spec/model-linking/tests.md, spec/native-lowering/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-092
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-033
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-082
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: reviews
---
# Spec review of quire-spec-language PR #654

## Summary

Ticket: QSL-642. Base spec review with the integrity, failure-domain and EARS lenses folded in, scoped to the diff. Judged against the decided rulings: the i128 ceiling; a typed refusal at check (`IntegerOutsideI128`, `ill_typed`/`type-mismatch`); decimal strings on the IR wire; model intake keeps `inexact-integer` for wide JSON numbers; ScalarLimits are budgets; option (a), where counters are number-or-string, integer values are decimal strings at every magnitude and `UnsafeInteger` is deleted; FR-033-AC-6 waits on IR-662.

- Ticket acceptance coverage. u64::MAX and i64::MAX+1: check (FR-091-AC-36), refine (FR-082-AC-9), key (FR-092-AC-14, T13/T14), replay (FR-098-AC-8), lower (FR-033-AC-6, i64::MAX+1 only). i128::MIN/MAX: key (T15), lower and IR round-trip (FR-033-AC-6), replay (FR-098-AC-8). i128::MAX+1 refused at check (FR-091-AC-37). Refinement with a literal above i64::MAX (FR-082-AC-9). IR wire round-trip at i64::MAX+1 (FR-033-AC-6). Counters at 2^53-1 and 2^53 (FR-092-AC-15). FND-003, FND-005 and FND-006 cover the gaps.
- Option (a) conformance. FR-092's statement and AC-15 match the ruling. AC-15 says `NodeKeyRefusal` has no magnitude variant, so `UnsafeInteger` is deleted. The current code confirms the premise: `UnsafeInteger` guards only counters (qsl-semantics/src/check/node_key/mod.rs:1429,1484-1487).
- Vectors T13, T14, T15 (and the existing T4) were recomputed with python hashlib over the RFC 8785 bytes. Each preimage is already canonical, and each key reproduces. T13 equals QSpec #192's `integer-range-u64-maximum` digest (`f9ea36c5...`), so the two repos agree.
- IR dependency: FR-033 names IR-662 for the i128 `IntegerType`/`IntegerLiteral`, and TC-912 and both matrix rows say it waits on IR-662. The other new ACs need no IR change. FR-056's string operand is allowed by filament-core-data's semantic-IR schema (`operands.value` is number or string).
- TC numbering: TC-909 to TC-913 don't collide with main or any open PR, including #645's TC-905 to TC-907. The AC ids are another matter (FND-002).
- Spec validation (`quire validate --scope . 'spec/**/*.md'`) gives output identical to origin/main: the same 2 pre-existing failures (TC-470, tests.md TC-202 row) and no new warnings on the new statements.

Examined:
- FR-091 "Integer bounds and literals up to i128" section, catalog row, FR-091-AC-36, FR-091-AC-37 (examined)
- FR-092 counter/value statement, T13 to T15, FR-092-AC-14, FR-092-AC-15, Status (examined)
- FR-033 two new statements, FR-033-AC-6, IR-662 dependency (examined)
- FR-056 Integer scalar-bound paragraph, FR-056-AC-16 (examined)
- FR-082 field-domain paragraph, FR-082-AC-9 (examined)
- FR-098-AC-8, Status (examined)
- TC-909, TC-910, TC-911, TC-912, TC-913 and their matrix rows (examined)
- qsl-semantics/src/check/lowering.rs negate keying, src/lowering.rs and src/lowering/wire.rs (context_only)
- open PR #645's FR-098 rows (context_only)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The unary-minus rule admits `-170141183460469231731687303715884105728` as i128::MIN, but never says what checked form it takes. Today check keys `-n` as a `quire.op.integer.negate` application over the literal `n` (check/lowering.rs:2890). Lowering emits `NumericNegate` over an `IntegerLiteral` (src/lowering.rs:584-588). So the operand literal is 2^127, which is outside i128. That breaks FR-091's own "every integer literal lies in i128", FR-033's "carry every integer literal value as an exact integer in i128" and IR-662's i128 `IntegerLiteral`. FR-033-AC-6 also wants a wire literal spelled `"-170141183460469231731687303715884105728"`, which only a fold produces. State the form: either fold a negated literal into one literal at check (and say whether that applies only to 2^127, or to every negated literal, which would change existing keys), or keep the literal unsigned and widen the literal's carrier. Then make FR-092 (key), FR-033 (wire) and AC-36/AC-6 agree with it. | spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md:678-681, spec/functional/FR-033-lower-bounded-integer-ir.md:44, spec/functional/FR-033-lower-bounded-integer-ir.md:66 |
| FND-002 | medium | FR-098-AC-8 has the same id as an AC in open PR #645 (QSL-640, head 6ec5705d9). That PR adds FR-098-AC-8 to AC-10 (composite and leaf-family replay). Whichever PR merges second gets a duplicate AC id or a conflict that someone resolves by hand. Renumber this AC (for example FR-098-AC-11), and update TC-913 and the tests.md row to match. | spec/functional/FR-098-execute-a-replay-request.md:155 |
| FND-003 | medium | The ticket's "FR-092 vectors at 2^53-1 and 2^53" have no recorded key anywhere in QSL. FR-092-AC-15 and TC-910 step 3 assert only byte fragments (`"size":9007199254740991`, `"size":"9007199254740992"`). The keyed vectors exist only in QSpec FR-322-AC-56 (`recursion-size-safe-maximum` `bc6fe35a...` and `recursion-size-two-pow-53` `89ec1ac8...`), and nothing ties QSL's encoder to them. Make AC-15 say that QSL's key encoder reproduces those two QSpec vectors' preimage bytes and digests, the same way AC-14 ties T13 to T15. | spec/functional/FR-092-key-type-parameter-and-declared-nodes.md:982 |
| FND-004 | medium | "every integer literal in an expression" is broader than the ruling, which covers integer type bounds and integer literals. As written, it also caps the integer arguments of `rational(n, d)` (FR-093 lists an integer literal and `rational(n, d)` as literal forms) and the unit-scale arguments at i128. Rational and decimal values are exact at any size today. Two implementers would read this differently: one refuses `rational(10^40, 1)` with `IntegerOutsideI128`, the other admits it. State whether the cap applies only to literals of integer type, or to every integer token. | spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md:666-668 |
| FND-005 | low | Two acceptance items are covered by proxies, and the AC doesn't say so. The ticket's first item names `Int[0, 18446744073709551616]` (2^64), which no AC uses (they use u64::MAX = 2^64-1). Its third item, "CG's wide-range model, written in QSL source, replays", is met by synthetic one-function units, and the CG model (IR-624 AC35) is never named. Add a 2^64 case to FR-098-AC-8 (or FR-091-AC-36), and name the CG model or state that the synthetic unit stands in for it. | spec/functional/FR-098-execute-a-replay-request.md:155 |
| FND-006 | low | The ticket's added acceptance says u64::MAX must "check, refine, lower and key". FR-033-AC-6 lowers only i64::MAX+1 and the i128 extremes, so no AC lowers `Int[0, 18446744073709551615]`. Add the u64::MAX bound and literal to AC-6 and TC-912. | spec/functional/FR-033-lower-bounded-integer-ir.md:66 |

## Verdict

Not mergeable as written: one high, three medium and two low findings. The overall design is sound. Values stay strings, counters switch at 2^53, the check-time refusal names site, value and limit and nothing reaches IR, ScalarLimits stay budgets, and the IR dependency is named. The vectors reproduce and validation matches main. FND-001 (the i128::MIN literal has no stated checked, keyed or lowered form) must be fixed. Fix FND-002 to FND-004 in the same round, and the lows with them.

## Dispositions

Round 1, reviewed at 15d426853e1dbc3a3f89b4b94312c4a558e50c0f. Recomputed: T16 `811c7208...` and L7 `c3aa30b2...` reproduce from their preimages (RFC 8785 bytes, python hashlib), and L7 has L2's shape with only `value` changed. `quire validate` output is identical to origin/main.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 15d426853 |
| FND-002 | fixed | 15d426853 |
| FND-003 | fixed | 15d426853 |
| FND-004 | fixed | 15d426853 |
| FND-005 | fixed | 15d426853 |
| FND-006 | fixed | 15d426853 |

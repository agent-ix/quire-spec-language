---
id: SR-15490
title: "integrity review of STD-262 FR-290 plugin result readers and replay context (QSL PR #1217)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@8e62c04067d50955f5388df4071fb2077f0a16b2; spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md, spec/test-cases/TC-774-plugin-results-settle-as-typed-records-and-plugin-proofs-carry.md, spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-290
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-774
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: reviews
---
# SR-15490: integrity review of STD-262 FR-290 plugin result readers and replay context (QSL PR #1217)

## Summary

Ticket: STD-262. Diff reviewed: `git diff origin/main...8e62c0406` (origin/main 9d340c0be). Sources
checked read-only: QSpec main 429c59d (FR-331 text and `proposals/backend-provider-v1/schema.json`,
FR-272, QSpec FR-290 capability-kind vocabulary, FR-382, FR-385, the checked-package v2 `Rational`),
QSL main FR-289 (after #1208), ADR-029 PL-7 and PV-4 (after #1210), FR-062, FR-070, FR-071, FR-098,
FR-395, ADR-013 O-09/O-19/O-25, and the qsl-replay code.

Checks that passed:
- The worked preimage of FR-290-AC-13 was recomputed: the JCS text matches byte for byte, its SHA-256 is
  `18ef6c73…eda2c3`, and with `entries: []` it is `9d60d63d…19ec14`. Both match the spec.
- The basis and cause spellings, payload members and counts match the FR-331 schema. The plugin bases
  match PL-7 (five) and the core-only bases match FR-289 (six, `kani_vacuous_proof` underscored). The
  accepted, Unsupported (8) and QSL-only (3) cause sets together cover FR-331's 26 inconclusive causes.
  The `reduction-not-preserving` reductions (`symmetry`, `partial-order`, `constraint`) and the
  lowest-terms `Rational` match their sources.
- Every cited FR-272 code and cause exists: `invalid_result`/`unknown-cause` and `missing-member`,
  `unknown_member`, `malformed_wire`, `unknown_required_feature`/`unsupported-feature`,
  `noncanonical_wire`/`inexact-integer` and `missing_declaration`/`missing-name`.
- The FR-289 ACs and the new ACs agree: FR-289-AC-13's `kani-vacuous-proof` passes
  `read_plugin_cause_kind` and is UnknownSpelling for the new reader, which is consistent.
- Code facts: `ProofBasis` derives only `Serialize`. `read_plugin_cause_kind` is as described.
  `qsl-replay` has no `qsl-route` dependency. `CallerLimits::spine` and `::accounting` are fields.
  `PluginResultRefusal` is `Copy` today, and the new variants will drop that. All 15
  `ReplayRequestWire` members are in the context table.

Lead defaults: 1, 2, 4, 9, 10 and 13 are sound. 3, 5, 6, 7, 8, 11 and 12 are sound in substance, but each
has a minor finding in this file or in SR-15491/SR-15492. The open replay-semantics point is FND-001.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | major, needs an owner/driver decision. FR-290-AC-12 requires `refuted` for a claim that holds. For `p(x: Int[0, 9])` returning `x + 1 > 3`, the value-validity item of `x + 1` says the application is defined and its result lies in its result bound (FR-062 "The claim", QSpec FR-290's row). That is true for every x. Yet the assignment `x = 0` makes `p` return `false`, so FR-098's predicate replay "reproduces the violation" and AC-12 settles the item `refuted`, naming the plugin. The opposite case also fails: a real value-validity counterexample is an undefined or out-of-bound application, which replays `NoValue` and settles `inconclusive`. FR-062 also gives an owner only to value-validity records, so every item that `request` accepts has this mismatch. A non-Boolean owner refuses `NotAPredicate`, and a synthesized owner cannot be called by name. Read literally, PL-7 and FR-289 give this result, but it breaks FR-289 item 4's intent, which is that `refuted` comes only from a reproduced counterexample to the item's own claim. The owner should decide between two options. (a) Plugin assignment replay covers only predicate-shaped items, and a value-validity counterexample settles `inconclusive` or is refused. (b) A replay mode checks the application occurrence, such as definedness and bound at the occurrence, or an FR-357-style value entry. Until then, AC-12 must not require `refuted` for an application item. | spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md:362 |
| FND-002 | low | minor: the `obligation_kind` member is "the record's FR-290 capability kind spelling". In a QSL spec, an unprefixed FR-290 is this requirement, and it defines no capability kind. The vocabulary is QSpec FR-290, and QSL's claim-form table is FR-057. Write "QSpec FR-290". | spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md:271 |
| FND-003 | low | minor: ADR-013 O-09's Public type row still lists the subject kinds as three untagged ones plus "a fourth subject kind" (the per-step record, tagged). The PR changes only "Who computes it". The `process-provider` preimage is a fifth, tagged subject kind, and the row's inventory does not mention it. Add it to the row. | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:296 |
| FND-004 | medium | minor: `bounded-solver`'s rule refuses an unknown `method` "because QSL decides soundness from the method kind (FR-250)". It then accepts `flowpipe-unrounded`, which `SolverMethod` documents as "no result of it is sound", as a `proved` basis. The PR body's lead default 7 (accepted because the proof is trusted) is not written in the spec, so the text contradicts its own rationale. Either state that `flowpipe-unrounded` is accepted on trust (PL-7) or refuse it as `Unsupported`. | spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md:146 |
| FND-005 | low | minor: a `certification` other than `trusted` is refused as `UnknownSpelling`, which maps to `invalid_result`/`unknown-cause`. FR-272 defines that cause's detail as "the received spelling of the result cause or proof basis", and `certification` is neither. `certified` and `uncertified` are also known FR-331 values, so calling them unknown spellings mislabels them. The sentence "The record carries `trusted` whatever the entry says" also contradicts the refusal. Give a non-`trusted` value its own refusal kind, or note the FR-272 stretch as a QSpec follow-up, and reword the sentence. | spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md:149-154 |

## Verdict

Not mergeable: FND-001 (major) needs an owner or driver ruling on replay semantics for application
items. The ruling should come before QSL-977 implements AC-12. FND-002 to FND-005 are minor. Apart from
these, the PR is consistent with FR-331, FR-272, FR-289, PL-7 and the code.

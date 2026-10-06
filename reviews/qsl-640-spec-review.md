---
id: SR-1345
title: "Spec review of quire-spec-language PR #645: composite WitnessValue replay and bounded_shadow Proved-vs-Tested against the QSL-640 rulings"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@a7cc42bba8caf9684d6ef18bcca4087c3615a93b; spec/functional/FR-070-*.md; spec/functional/FR-098-*.md; spec/functional/FR-263-*.md; spec/functional/FR-358-*.md; spec/test-cases/TC-736-*.md, TC-905-*.md, TC-906-*.md, TC-907-*.md; spec/spec.md; spec/tests.md; context: spec/decisions/ADR-013 (O-09, O-16, C-09), ADR-014 (§1 B-4, §2, §4, §8), ADR-021 (TX-2, TX-3), FR-071, FR-255, FR-277, qsl-replay/src/witness.rs, qsl-replay/src/proof_result.rs; agent-ix/quire-contract-codegen@ae390fa spec/kani/functional/FR-015, FR-025, FR-028; agent-ix/quire-specification origin/main spec/functional/runtime/FR-181"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-070
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-263
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-358
    type: reviews
---
## Summary

Ticket: QSL-640. The rulings are the ticket body plus its 2026-10-05T22:25
comment, judged as decided. Sub-analyses folded in: integrity, failure-domain,
scope-boundary (CG seam). `quire validate` passes on every changed file except
spec/tests.md, whose one failure (TC-202's Status cell) is already on main and
outside this diff.

What matches the rulings:

- **Wire form and decode (FR-070).** The value text is JCS of QSpec FR-181's
  typed canonical form for option, record, tuple, union, sequence, set, bag and
  ordered set, with `Boolean` and `i64` leaves. The one deviation, a
  declaration node id in `name` in place of FR-181's `<qualified-name>`, is
  stated and justified (FR-181 leaves that text form to the implementation).
  Decode accepts any depth (FR-261 reader, explicit heap stack, ADR-030 D-1).
  Every walk (clone, eq, Debug, drop) is heap-stack. `replay.input_bytes` is
  the only bound. Non-canonical bytes refuse, so the value text is canonical.
- **Admission (FR-098).** Composite values convert against the recompiled
  package's types and are admitted through S6a under FR-106's input rule. An
  out-of-domain leaf or cardinality refuses `WrongValueKind`
  (`invalid_runtime_input`) before the call. FR-321 governs unions.
- **Falsified items (FR-358).** These settle through `qsl_replay::replay` only:
  Refuted, ReplayParity or ReplayRefused, and a fault settles Failed, as
  ADR-013 C-09 and the existing `TerminalValue::from_replay_refusal` give.
  Out-of-domain input is refused, and a post-state out of range is evidence
  (QSL-634).
- **Verified items (FR-358).** `Proved{n}` iff the harness bounds cover every
  declared bound, otherwise `Tested`, and 0 checks is the vacuous
  `Proved{success_checks: 0}`. This matches the ruling and the existing
  `TerminalValue`.
- **FR-263-AC-1 / TC-736.** The criterion is moved to QSL-640 at 100,000 long,
  with `replay.input_bytes` raised. The nested-record case is FR-098-AC-8 /
  TC-906.

Specific points from the brief:

1. **ADR-021 citation.** The author is right. ADR-021 TX-2 is about symmetry
   transfer: code-side evidence that relies on symmetry closure needs either no
   observable identity order or CG ADR-003 Q1's bounded shadow path. TX-3 says
   a harness identity carries a reduction only as "a tightened bound recorded in
   the identity". ADR-013 O-09 is the obligation identity. ADR-014 B-4 is a
   proof bound that "is part of the obligation identity (O-09)". FR-358 cites
   TX-3, O-09 and B-4 for bounds in the identity, which is correct. Its gloss
   of TX-2 ("names the bounded shadow path as the code-side evidence route")
   is a little broad, but it is not wrong. The IR-635 relay already carries the
   correction.
2. **Trusted declared bounds.** At this head this is a soundness gap (FND-001),
   and the plan lead's ruling (a) resolves it: QSL recompiles from an FR-071
   request tied to `package_id`, and CG supplies only harness bounds and
   success_checks. FND-001 lists what the amendment must also close.
3. **CG implementability.** The form is implementable. CG's planned leaf draw is
   FR-015-AC-70 (IR-264): a bool per Boolean leaf, an i64 per bounded-integer
   leaf, a presence bool per option, and present/absent/null per optional
   record field, ordered ascending by parameter node id and then leaf path. It
   maps one to one onto the record, tuple and option shapes, and the
   declaration keys CG needs for `name` are in its identity (FR-015-AC-76). CG
   needs a JCS writer (quire-canonical). Two things belong to IR-635, not this
   PR. First, CG FR-025 on main still marks composite and collection
   `unsupported` (FR-025-AC-7), so IR-635 must amend it. Second, CG's shadow
   has no collections, unions or recursion (FR-015's unsupported-shape table),
   so QSL's form is a superset of what CG emits today. That is fine. FND-003
   covers what a falsified composite_equality item means.
4. **Leaf families and delimiters.** The exclusion is stated: FR-070 says
   leaves are Boolean or integer, FR-098 lists the kinds that have no
   `WitnessValue`, and decode refuses an unknown tag (tested with `text`).
   The delimiter claim holds only because text is excluded (FND-006). Ruling
   (b), all leaf families, makes that a live hole for the amendment.
5. **Canonical uniqueness.** The value text is unique: decode refuses anything
   that is not its own JCS, set and bag order is by JCS bytes, set and
   ordered-set duplicates refuse, and FR-181 decimal strings forbid `-0` and
   leading zeros. The FR-070 sentence claims more than that: transcript entry
   order is not canonicalised (FND-007). For the amendment: FR-181 keeps a
   decimal's retained coefficient and scale (`1.0` and `1.00` differ) and a
   float's bits (`+0` and `-0` differ). Byte-level set and ordered-set
   duplicate checks must agree with the language's equality for those
   families, or the amendment must say they do not.
6. **Limit accounting.** Using `replay.input_bytes` as the only decode bound is
   consistent with FR-263-AC-3 and FR-255. The pre-call check against
   `value_occurrences` and `work_units` is not (FND-002). Those are B-2
   accounting limits, and FR-098 refuses them as a B-3 `LimitExceeded`
   "naming that counter as its setting (FR-255)". FR-255 has no such setting,
   and FR-277 gives `Incomplete` for execute's accounting limits. Not charging
   the meter, so the call's charges match the proving run, is a sound choice
   for parity. It still needs to be stated against FR-277, which it now
   silently departs from.

## Verdict

Changes requested, spec text only. The rulings are carried faithfully for the
wire form, decode, admission and the falsified and verified settlements. Two
high findings remain. The verified settlement trusts caller-supplied declared
bounds and the completeness of the caller's bound list, and the amendment from
ruling (a) is in progress. The pre-call limit checks refuse B-2 accounting
limits as B-3 stage limits with settings that do not exist. Four medium
findings concern the composite_equality falsification meaning, caller-
substituted B-4 bounds always settling Tested, an undefined precedence between
the vacuous result and the refusals, and the text-leaf delimiter hole that
ruling (b) opens.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `settle_verified_shadow` takes the declared bounds and the bound list from CG ("CG passes the declared and harness bounds its identity records"), with no recompile, no `package_id` tie and no check that the list covers every bounded or unbounded position. A caller can get `Proved` over a domain it did not prove by (a) stating a declared bound narrower than the source, (b) leaving a position out (the "empty bound list is covered" rule then gives `Proved` for a composite with `Int[0, 9]` leaves), or (c) passing harness bounds looser than the ones that ran. QSL owns the verdict (CG AD-002), so this is a soundness gap. Resolved by plan-lead ruling (a) pending amendment. The amendment must: derive the position set and the declared bounds from the recompiled package (FR-071 request plus `package_id` check), using ADR-014 §4's extent walk over the transitive closure of the parameter types; refuse, never settle `Proved`, on a missing harness bound for a derived position and on a harness bound for a position that does not exist; tie the harness bounds to the request's `obligation_identity` (recompute the O-09 identity and compare); and apply the "empty list is covered" rule only to the derived set. | spec/functional/FR-358-settle-a-composite-bounded-shadow-item.md:63-69, 120-121 |
| FND-002 | high | FR-098 refuses a composite argument whose occurrence or node count reaches the accounting limit `value_occurrences` or `work_units` with FR-277's `LimitExceeded` (`stage_limit_exceeded`), "naming that counter as its setting (FR-255)". These are `quire.value.accounting/v1` ceilings, which are B-2 wherever they are read (ADR-014 §1 classification rule). FR-277's Outputs give `Incomplete` for execute's accounting limits. FR-255's closed settings table has no `value_occurrences` or `work_units` setting, so the refusal names a setting that does not exist, and FR-098-AC-9 / TC-906 step 3 test it. Fix: either add replay-reader B-3 settings to FR-255 (for example `replay.value_occurrences` and `replay.value_nodes`, with defaults) and refuse with those, or classify the pre-check as B-2 and settle it as `Incomplete`. In either case state that the pre-check charges nothing, so the call's meter matches the proving run. | spec/functional/FR-098-execute-a-replay-request.md:149-160 |
| FND-003 | medium | FR-358's falsified half assumes a predicate obligation ("the selected `Boolean` function") whose violation the replay reproduces. The one composite bounded_shadow family planned in CG, composite_equality (CG FR-015-AC-69 to AC-76, IR-264), asserts that the generated shadow's verdict and pair count equal QSpec FR-149's expectation. A falsification there is a shadow or oracle defect, not a counterexample to the user's function, and settling it `Refuted` (or `ReplayParity`) would report a tool defect as a property violation. Fix: say which composite bounded_shadow items carry a predicate whose falsification replays, and what a falsified shadow-refinement item settles to (for example `Failed`, as C-09 treats a tool defect). Otherwise scope FR-358 to predicate items and name the owner for composite_equality. | spec/functional/FR-358-settle-a-composite-bounded-shadow-item.md:59-62, 80-100 |
| FND-004 | medium | FR-358 says a declared bound of `None` is never covered, so a harness bound over an unbounded domain always settles `Tested`. Under ADR-014 §4's bounded request, the caller substitutes a B-4 `ProofBound` for each unbounded domain. The new item is `Bounded`, its identity carries the bound (O-09), and §8 makes its result evidence for exactly that set of ProofBounds, settled against its own request_index. In replay, `DeclaredDomain` holds that `ProofBound` (ADR-014 §1). FR-358 gives no way for a requested bounded item to settle `Proved` over its requested bounds. Fix: define the declared bound as the request's `DeclaredDomain` (the substituted ProofBound where one exists, the authored bound otherwise), which fits ruling (a)'s FR-071 input. Or state that B-4 bounded composite items settle `Tested`, and amend ADR-014 §4/§8 to match. | spec/functional/FR-358-settle-a-composite-bounded-shadow-item.md:116-119 |
| FND-005 | medium | The Verified rules conflict when `success_checks` is 0 and the bounds are defective. One rule says to return `Proved { success_checks: 0 }` "whatever the bounds". The other says to return `ShadowBoundRefusal` for a duplicate `DomainKey` or a kind mismatch. No precedence is given, and FR-358-AC-6 tests the two cases separately, so two implementers can differ. Fix: refuse first (a defect in the caller's identity), then apply vacuity, and add the combined case to AC-6 and TC-907 step 4. | spec/functional/FR-358-settle-a-composite-bounded-shadow-item.md:104-105, 124-128 |
| FND-006 | medium | FR-070 rests the transcript's integrity on "every JSON string ... holds none of `;`, `=`, `<` or `>`". That holds only while text is excluded. JCS escapes only `"`, `\` and control characters, so a text leaf `"a;b"` would split in `Witness::raw_bindings` (`split(';')`), and `">>>"` would end the block in `find_blocks`. A quantity's `unit` `<qualified-name>` has no fixed text form either. Ruling (b) adds every leaf family, so the amendment must close this. It must do so with one canonical escape (decode refuses any other spelling, so uniqueness survives), count `replay.input_bytes` on the escaped bytes, and give a criterion over `;`, `=`, `<<<` and `>>>` inside a text leaf. | spec/functional/FR-070-implement-typed-counterexample-witness-envelope.md:115-119 |
| FND-007 | low | FR-070 says refusing non-JCS value text means "two equal witnesses always have identical transcripts (ADR-014 TR-1)". That is true of each value text, not of the transcript. Entry order in the values field is not canonicalised, and decode joins by node id and accepts any order. Fix: scope the sentence to the value text, or require entries in ascending parameter node id (CG FR-025 already orders arguments that way) and refuse any other order. | spec/functional/FR-070-implement-typed-counterexample-witness-envelope.md:149-151 |
| FND-008 | low | The spec.md row for FR-071 is edited to say "composite witness values ... specified, not yet implemented -- TC-905 and TC-736 planned", a copy of FR-070's new row text. FR-071 is unchanged in this PR, and neither TC-905 nor TC-736 verifies FR-071. Fix: revert the FR-071 row, or amend FR-071 if it should own part of the change. | spec/spec.md:989 |
| FND-009 | low | FR-358-AC-3 says a value text with a `text` tag "settles `Inconclusive(ReplayRefused)` with that refusal's code" but never names the code. A wrong code passes the AC and TC-907 step 1. Fix: name the catalog code a composite `DecodeRefusal::Malformed` replay refusal carries. | spec/functional/FR-358-settle-a-composite-bounded-shadow-item.md:136 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-010 | low | The spec/tests.md row for TC-905 lists FR-070-AC-8, AC-9 and AC-11 but not FR-070-AC-12 (entry order), which TC-905's Scope line and trace tags include. The matrix therefore shows AC-12 uncovered. Fix: add FR-070-AC-12 to the TC-905 row. | spec/tests.md:536 |
| FND-011 | low | FR-070 now refuses witness transcript entries that are not in ascending parameter node id order (`DecodeRefusal::EntryOrder`). FR-098 Behavior still says the executor orders the arguments "whatever order they arrive in", and FR-098-AC-2 says `Witness` bindings join "whatever order they arrive in". A reader can take that as permission for unordered transcript entries. Existing multi-entry transcripts in TC-444 fixtures may also be out of order. Fix: in FR-098 Behavior and AC-2, say that `Input` assignments and the binding list may arrive in any order, while witness transcript entries must be ascending (FR-070-AC-12). | spec/functional/FR-098-execute-a-replay-request.md:105-110 |
| FND-012 | low | An integer leaf inside a witness value text is now unbounded (FR-070-AC-11 decodes `-170141183460469231731687303715884105728`). A scalar `Integer` parameter keeps its `I64` entry, so the same value cannot be written for a top-level parameter. FR-070 also does not name the `WitnessValue` leaf that holds an integer outside `i64` (today `WitnessValue::Integer(i64)`). Fix: name the unbounded integer leaf (a new variant or a widened one), and either state that the top-level `I64` form stays `i64` because harness arguments are `i64` (CG FR-025), or let an unbounded `Integer` parameter take a `Canonical` entry. | spec/functional/FR-070-implement-typed-counterexample-witness-envelope.md:140 |
| FND-013 | low | FR-358 step 2 derives positions only for ADR-014 B-4 kinds: integer range, cardinality and depth. Text length, decimal range and scale, rational, float and quantity derive none. A future shadow that narrowed one of those (for example, drawing `Text[0, 8]` only up to length 4) would still settle row S-3 `Proved`, because no position goes uncovered. CG's shadow refuses those families today (CG FR-015 unsupported-shape table, `ShadowFamilyNotBuilt`), so nothing settles wrong now. Fix: state that a verified item over a leaf family with no B-4 bound kind settles `Proved` only when the harness draws that leaf over its whole declared domain, and otherwise `Tested`. Or refuse such items until ADR-014 has a bound kind for that family. | spec/functional/FR-358-settle-a-composite-bounded-shadow-item.md:173-189 |

## Dispositions

Round 1, reviewed at 8b61f6f2d238bc20f89893adf96941b875aab519 (`git diff a7cc42bb 8b61f6f2d`: b915f0982, 66ea335ac, 8b61f6f2d). Spec only, no build. `quire validate` passes on every changed file except spec/tests.md's TC-202 Status cell, which fails on main too.

Author's open point 3 (float elements in sets): S3 does not admit them. QSpec FR-144-AC-6 makes an IEEE-bearing set, bag or ordered-set element type `ill_typed` at any depth. QSpec TC-194 E28 refuses `Set<Float64>[0,2]` as `operator-ineligible`, and QSL FR-323-AC-3 refuses `Set<F>` over a `Float64` union payload. No declared type can hold `+0` and `-0` as set elements, so the spec needs no signed-zero rule. A decoded float set fails FR-098's conversion as a kind mismatch. One sentence in FR-098's duplicate bullet saying so would help a reader, but it is not a defect.

Routed, not a finding: whether FR-358 step 5's recomputed O-09 preimage (function node id, `declaration` occurrence key, obligation kind, arguments with harness bounds) lines up with the extra identity members of CG FR-015-AC-76 (closure declaration keys, operator, size budget, abstractions). The coordinator has routed this to the QSL plan lead. ADR-013 O-09 itself names "each a parameter node id and its declared domain" for `arguments`, so the same ruling should also say whether the harness bound replaces the declared domain there (ADR-021 TX-3).

Notes on the fixes:

- FND-001: QSL now recompiles, ties `package_id`, derives positions and bounds itself, refuses stray, duplicate and mis-kinded bounds, and ties the harness bounds to the O-09 identity. A missing harness bound is not covered, so the item settles `Tested`. The empty list covers only an empty position set.
- FND-002: the pre-check is now B-2 and settles `Incomplete`, then `inconclusive`/`NoValue`. It deliberately charges nothing.
- FND-003: the shadow-refinement kind is added; `Disagreed` settles plain `Failed` (row S-1).
- FND-004: the declared bound is the request's `DeclaredDomain` where one exists. AC-4 covers the substituted cardinality-5 case.
- FND-005: refusals come before row S-2 (AC-7).
- FND-006: four-character percent escape with one spelling. AC-11 covers it, and escaped bytes count toward `replay.input_bytes`.
- FND-007: entry order is canonical (AC-12).
- FND-008: FR-071 row reverted.
- FND-009: AC-3 names `invalid_runtime_input`.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 8b61f6f2d |
| FND-002 | fixed | 8b61f6f2d |
| FND-003 | fixed | 8b61f6f2d |
| FND-004 | fixed | 8b61f6f2d |
| FND-005 | fixed | 8b61f6f2d |
| FND-006 | fixed | b915f0982 |
| FND-007 | fixed | 8b61f6f2d |
| FND-008 | fixed | 8b61f6f2d |
| FND-009 | fixed | 8b61f6f2d |

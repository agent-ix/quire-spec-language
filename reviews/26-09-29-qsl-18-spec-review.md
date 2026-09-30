---
id: SR-786
title: "Spec review of QSL-18 sum/case mapping (ADR-012 §16)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@c6e1e5ac; spec/decisions/ADR-012-semantic-family-extension-contracts.md (§16 and the QSL-18 amendments to Status, §2, §3, §12.1, §14.1, Consequences); context read: ADR-011 §6.1 and §7.3, ADR-013, FR-057, FR-091, FR-092, docs/family-migration-recipe.md, the cited QSL code at HEAD, and QSpec origin/main (AD-015, FR-141, FR-143, FR-144, FR-146, TC-262 to TC-265, shared-grammar.md, native-diagnostics.md, value-accounting.md, checked-package-v2/schema.json)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: reviews
---
## Summary

Ticket: QSL-18 (#221, ARCH-41). Branch `spec/18-arch41-mapping` at c6e1e5ac.
The review subset is the base checklist adapted to an ADR section, plus the
object, integrity, failure-domain, interface/scope-boundary and evidence
analyses. Claims were measured against the tree at HEAD and QSpec
`origin/main`. The draft was not trusted.

Sound:
- The family assignment in §16.2 matches ADR-012 §1 and §3. `enum` stays
  `Value`'s (FR-091:880). `SumCase` reads only `Value`'s checked types.
  `value::declaration` is the layer-3 `semantic_value` registry
  (ADR-011:870; the file's own module doc). So joining union payloads to
  `check_recursion` is shared code, and it creates no `SumCase` to `Value`
  family edge.
- Removing `SumCase` from S6a is consistent with ADR-012 §5.1 ("one variant
  per family that implements `ReferenceEvaluation`") and ADR-013 O-16
  (:450-453). `case` needs no `FamilyResult` cause.
- Gaps SC-G1 to SC-G5 are real. On QSpec `origin/main` the schema has no
  `union` in `CompositeTypeNode.semantic_form` and no `case` or union
  construction in `ExpressionNode`/`ValueNode`. `OperationMember` has exactly
  the eight kinds listed. FR-141:81 gives a preimage for enum members only.
  value-accounting.md:38 names record fields and tuple arguments only.
  FR-144:109-120 has no union row.
- FR-146:74-79 confirms the obligation order and names. The catalog cause
  `unproved-exhaustiveness` exists from `1-draft.4` (native-diagnostics.md:85,
  :132-134), and the catalog is at `1-draft.8`. FR-143:58-66 confirms the four
  construction refusals as `ill_typed`/`type-mismatch`, and FR-143:92-97
  confirms the foreign-key input refusal. The AD-015 recursion rows match
  §16.8.
- FR-057:173-174 confirms both claim-form rows (`value-validity`, and none for
  the obligation).
- These ADR-013 citations are confirmed: O-04, O-06, O-07, O-12, O-13, the
  O-14 text quoted in SC-Q1, O-16, O-26, QC-15, QC-19, C-07, C-14, C-26,
  C-30, R-02, R-08, T-2, T-4 and T-5.
- The M-6e claim holds: SEAM-2 has no sum/case code, and the `case` hits in
  `src/` are the protocol `choice`. The §15.4 STD-111 precedent is real
  (ADR-012:1283-1288), although "minted over the proposed spelling" is an
  inference from it.
- The new ids SC-R1 to SC-R5, SC-G1 to SC-G5 and SC-Q1 are unique in the repo.
- Most code citations are exact: `infer_form` :613, :637 and :783; `Machine::apply`
  :929; `evaluate_declaration` :273; `lower_node` :2739; the `NodeKind`,
  `NodeTag`, `SemanticTerm` and `Operator` enums; `refusal.rs` :19, :234,
  :561, :668, :868, :874, :884 and :926; `family/mod.rs` :90, :96 and :128;
  `token.rs` :138 and :159; `grammar.rs` :1205-1213; `dispatch.rs` :159, :415
  and :460-472; `syntax.rs` :253 and :1496; `declaration.rs` :682-694 and
  :1160; `emit.rs` :194, :393 and :780; `execute.rs` :259; `seam_probe.rs`
  :1122; `lib.rs` :38; `quire-exact` `value.rs` :87 and :767. The exceptions
  are listed in FND-010.

Per analysis:
- **Base.** The ids are unique. The citation slips are in FND-010 and FND-011.
  Stale contradicting sentences remain elsewhere in the ADR (FND-006).
- **Object.** The entities are complete except the `case` result type
  (FND-005) and the arm-order identity of the `case` node (FND-009).
- **Integrity.** The amendment leaves contradictions: ADR-012 :270, :899,
  :945, :956, :959 and :1090 (FND-006), and the migration recipe (FND-007).
  The refusal semantics under the fail-fast `Typer` are undecided (FND-004).
- **Failure domain.** An untrusted union argument can reach the S6a path that
  §16.4 calls `InternalFault` (FND-003). The canonical key before SC-G5 has
  no owner (FND-008).
- **Interface and scope.** The §16.7 allowed change set, which is closed by
  its own rule, misses required and compile-forced paths (FND-001). The
  "check-core resolver" does not exist, and today's classifier is `Value`'s
  `Application` (FND-002).
- **Evidence.** The test plan is bounded. It misses two §16.5 rows and the
  scrutinee-refused behaviour (FND-013).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | §16.7's allowed change set is incomplete, and §16.7 makes any path outside it a defect that reopens #221. So #187 reopens #221 on its first PR. Paths it must touch that are missing: (1) `qsl-cst/src/cst.rs`, the `Production` inventory. It needs new productions, and the expression `case` needs a name distinct from the existing protocol `Production::Case`. (2) `qsl-forms/src/value.rs`: its exhaustive `Production` matches get forced arms, and the CST-to-`Expression` builder (where `Expression::If` is built) must build `Expression::Case`. (3) `qsl-semantics/src/check/checked_dispatch.rs`: its exhaustive `Expression` rewrite (the FR-151 rename) is forced, and it must not rename arm binders. (4) `check/family.rs`: its exhaustive `Expression` walk is forced, and `Application` lives there (FND-002). (5) `check/check.rs`: the scope member index and `Typer::name` resolve nullary `U::m`. (6) If SC-Q1(a) puts unions in `TypeEnvironment.composites` (`check_recursion` only walks `composites`), every exhaustive `CompositeShape` match is forced, for example `family/requirements.rs`, `check/lowering.rs`, `qsl-eval` `evaluate.rs` and `qsl-package` `emit/extent_agreement.rs`. (7) Every test and spec path: `*/tests/it/*.rs`, new TC files and FR ACs. (8) `docs/family-migration-recipe.md` (FND-007). Failure: a #187 coder either stops to reopen #221, or routes the logic into listed files to stay "in set". Fix: add these paths. Or replace "only the paths below" with a rule: a compile-forced arm in any file of the listed crates, plus tests and spec. Keep the explicit Excluded list as the hard boundary. | spec/decisions/ADR-012-semantic-family-extension-contracts.md:1636-1648; qsl-cst/src/cst.rs:111; qsl-cst/src/grammar.rs:1206; qsl-forms/src/value.rs:683; qsl-forms/src/value.rs:960-1033; qsl-forms/src/value.rs:1461-1514; qsl-semantics/src/check/checked_dispatch.rs:1246; qsl-semantics/src/check/family.rs:603; qsl-semantics/src/check/check.rs:579; qsl-semantics/src/check/check.rs:1367; qsl-semantics/src/value/declaration.rs:1160-1175 |
| FND-002 | medium | The §16.2 construction dispatch describes a "`check`-core resolver" that returns a "closed `check`-core enum of resolved-target kinds" as if it existed. §16.6 lists it under "Each one already exists", located in "`check` core". Neither exists. Today the `Call` arm calls `Value`'s `Application::resolve` directly, which returns `enum Application { Function, Tuple, Imported }` in `check/family.rs`. Nullary enum members resolve in `Typer::name`. Two problems follow. (a) As written, the arm calls the resolver and then matches on the result. That is a branch in the arm, which ADR-012 §4.3 counts as semantic logic. (b) Resolution precedence is unstated. `Application::resolve` tries `q::m` as an import qualifier first, so a union named like an import alias, or like an enum, resolves ambiguously. Failure: #187 either adds `Application::Union`, a `Value` edit in an unlisted file, or builds a second parallel resolver. Fix: pick one design. Either extend `Application` (and say so, and list `check/family.rs`), or name a new single check-core seam function that the arm calls once, with one call per result arm. State the precedence (import, union, enum, function, tuple), and that a clash refuses as `AmbiguousName`. | spec/decisions/ADR-012-semantic-family-extension-contracts.md:1442-1468; spec/decisions/ADR-012-semantic-family-extension-contracts.md:1589; spec/decisions/ADR-012-semantic-family-extension-contracts.md:1598; spec/decisions/ADR-012-semantic-family-extension-contracts.md:529-537; qsl-semantics/src/check/check/typing.rs:783-794; qsl-semantics/src/check/family.rs:739-839; qsl-semantics/src/check/check.rs:1367-1403 |
| FND-003 | high | An untrusted union argument can reach the path §16.4 calls `InternalFault`. Argument admission (`validate`) relies on kernel `ValueType::admits`. For `Composite`, `admits` checks only the declaration key; the contents are "admitted at construction". SC-Q1's recommended option (a) gives `Value`'s union variant only a `VariantId` and payloads, with no union key. So `admits` cannot perform SC-R3's foreign-union check at all. Nothing in §16 refuses a supplied value whose `VariantId` is not a member of `U`, or whose payload arity or types are wrong. Such a value passes admission and matches no arm. §16.4 then says `Err(InternalFault)`, "never a refusal". That turns an input problem into an invariant break, against O-16 and T-4 (input problems are `InputRefusal`). A `Reference` nested in a union payload also skips the dangling-reference walk unless that walk descends into it. Fix: extend SC-R3. Admission refuses `invalid_runtime_input`/`wrong-value-kind` for a non-member `VariantId` and for a payload arity or type mismatch, and walks payloads for references. Have SC-Q1(a)'s value carry the union node key. Add the adverse admission tests to §16.8. | spec/decisions/ADR-012-semantic-family-extension-contracts.md:1498; spec/decisions/ADR-012-semantic-family-extension-contracts.md:1523; spec/decisions/ADR-012-semantic-family-extension-contracts.md:1758; qsl-eval/src/value/expression/mod.rs:176-229; quire-exact/src/value.rs:221-242 |
| FND-004 | medium | The §16.5 refusal semantics do not match the engine or ADR-012 §4.2. (a) The `Typer` is fail-fast: `infer_form` returns `Result<Step, CheckRefusal>`, and `FamilyContract::check` returns one refusal. §16.5 says a body refusal "is reported beside" the exhaustiveness refusal, which implies two refusals. Take a `case` whose arm 1 body is ill-typed and which also misses an arm. The oracle could be `ill_typed`, `missing-arm` or both, so the §16.8 locus and payload assertions are unstable. (b) §4.2 lets `finish` run a cross-clause check "only when every clause that check reads was checked successfully". §16.5 never says that `finish` reads arm heads only. (c) When the scrutinee is refused (not a union), there is no `U`. The draft does not say whether the arms and `finish` run. Fix: state that one refusal is reported per `case`. Typing comes first, per FR-146:264 ("typing checked first even in an unreachable branch"), then the first obligation. State that `finish` reads the recorded arm heads only, and that a scrutinee refusal ends the builder. Add one test vector for each. | spec/decisions/ADR-012-semantic-family-extension-contracts.md:1529-1544; spec/decisions/ADR-012-semantic-family-extension-contracts.md:474-527; qsl-semantics/src/check/check/typing.rs:613-619; qsl-semantics/src/family/contract.rs:370-373 |
| FND-005 | medium | The `case` result type is undefined. FR-146:90-92 checks each arm body against "the `case` expression's declared result type", but the grammar has no annotation on `case` (shared-grammar.md:327-330). §16.5 says "checks the body against the `case` result type" without deriving it. It could come from the `Want` hint, from the first arm, or from a join as for `if`. Failure: the "arm body not of the `case` result type" row has no deterministic expected type or locus, and two implementations disagree on which arm is wrong. Fix: decide the rule, for example the same rule `Expression::If` uses today, with the reference cited. File the missing QSpec definition as SC-G6. | spec/decisions/ADR-012-semantic-family-extension-contracts.md:1537; spec/decisions/ADR-012-semantic-family-extension-contracts.md:1554; QSpec spec/functional/expressions/FR-146-check-total-pure-functions.md:90-92; QSpec proposals/quire-v1/shared-grammar.md:327-330 |
| FND-006 | medium | The amendment corrects the §2 Stage hooks row and §12.1's Diagnostics row. Other sentences in the ADR still contradict §16. (1) :270: "`ReferenceEvaluation` is implemented by every family except `Relation`". (2) :899, the §11 #187 row: "reference evaluation in `SumCase`". (3) :1090, §13.5: "`evaluate` (every family except `Relation` …)". (4) §12.1 Form row :945: `ArmForm { pattern, body }`. (5) §12.1 Witness row :956: "replay through the `SumCase` `evaluate` arm". §16.4 (:1525) calls that row "unchanged", yet it contradicts §16.6. (6) :959: "the unreachable-arm refusal". The blanket "§16 holds" rule covers these, but a #187 coder reading §2 or the §12.1 tests paragraph gets the opposite instruction. Fix: amend these six places, and drop "unchanged" from :1525. | spec/decisions/ADR-012-semantic-family-extension-contracts.md:270; spec/decisions/ADR-012-semantic-family-extension-contracts.md:899; spec/decisions/ADR-012-semantic-family-extension-contracts.md:945; spec/decisions/ADR-012-semantic-family-extension-contracts.md:956; spec/decisions/ADR-012-semantic-family-extension-contracts.md:959; spec/decisions/ADR-012-semantic-family-extension-contracts.md:1090; spec/decisions/ADR-012-semantic-family-extension-contracts.md:1525 |
| FND-007 | medium | The migration recipe contradicts §16.6. Its "Required conversions" item 2, the text FR-066-AC-2 checks, requires every family except `Relation` to add an evaluator and an `S6aFamilyKind` variant. §16.6 and §16.7 forbid both for `SumCase`. The recipe is neither amended nor in the change set. Failure: #187 cannot satisfy both the recipe (FR-066-AC-2) and §16.7. Fix: amend the recipe in this PR, keying item 2 on "families whose declarations S6a calls or selects" as the new §2 row does. | docs/family-migration-recipe.md:52-60; spec/decisions/ADR-012-semantic-family-extension-contracts.md:1620-1632; spec/decisions/ADR-012-semantic-family-extension-contracts.md:1660-1662 |
| FND-008 | medium | Behaviour before SC-G5 has no owner. §16.7 has #187 add the forced union arm in `quire-exact` `key.rs`, but FR-144 defines no union key. §16.10 says only that set, bag and map use "depends on" SC-G5. Nothing says what S3 does with `Set<U>`, `Bag<U>`, `OrderedSet<U>` or `Map<U, _>` before then. Failure: the forced kernel arm panics, or invents a canonical order that QSpec later contradicts. That is a wrong result in collection algebra. Fix: have #187 refuse union element and key types at S3 with a named cause (owner `SumCase` or `Value`, stated) until SC-G5 lands. The kernel arm is then an explicit refusal. Add one adverse test. | spec/decisions/ADR-012-semantic-family-extension-contracts.md:1646; spec/decisions/ADR-012-semantic-family-extension-contracts.md:1731; QSpec spec/functional/type-model/FR-144-preserve-complete-collection-algebra.md:105-120 |
| FND-009 | medium | SC-R2 gives "an arm order that keeps the arm set" as a presentation-only edit, but asserts only that union node ids and `VariantId`s are unchanged. The `case` node is content-keyed (FR-093), and nothing decides whether its preimage orders arms by source position or canonically by member identity. Arm order does not affect meaning: there is no wildcard, and selection is by member. Under source ordering, reordering arms changes the `case` node id, every enclosing function node id and `package_id`. So SC-R2's example silently fails for the nodes a reader would expect it to cover. Fix: put a canonical arm order (by member identifier) into the SC-G1 proposal and assert the `case` node id in SC-R2. Otherwise remove arm order from the presentation-only examples. | spec/decisions/ADR-012-semantic-family-extension-contracts.md:1480; spec/decisions/ADR-012-semantic-family-extension-contracts.md:1497 |
| FND-010 | low | Citation slips. (1) §16.1 titles: FR-143 is "Evaluate records, tuples and finite recursive values" (FR-143:3), and "Declarations and identity" is its section at :67. FR-146 is "Check total pure functions and recursion" (FR-146:3), and "Case exhaustiveness" is its section at :66. (2) `d70cd64` is QSpec PR #121, which closes #115. (3) `checked_in_locations()` has 15 entries, not 16. (4) `S6aFamilyKind::family` maps `S6aFamilyKind` to `FamilyKind`; it is not a match over `FamilyKind`. `FamilyKind`'s only match is `catalog_code_prefix`, plus the const prefix assertion at :198-219. (5) `CheckedTypeNode::Sum` is at identity.rs:538, not :541. (6) `emit.rs:319-330` is the `SemanticTerm::Frame` arm; `BodyNames::of` is at :279. (7) The owner-absent keying rule is FR-092:87, not FR-093. Fix: correct these. | spec/decisions/ADR-012-semantic-family-extension-contracts.md:1399-1409; spec/decisions/ADR-012-semantic-family-extension-contracts.md:1480; spec/decisions/ADR-012-semantic-family-extension-contracts.md:1594; spec/decisions/ADR-012-semantic-family-extension-contracts.md:1605; xtask/src/seam_probe.rs:179-251; qsl-eval/src/value/expression/s6a/mod.rs:121-137; qsl-semantics/src/check/identity.rs:538; qsl-package/src/emit.rs:279 |
| FND-011 | low | Two §16.9 claims have no source. (1) The profiles bullet says a union is admitted wherever records and tuples are. No QSpec profile names `union`, and `quire.value.complete/v1` does not list it, so this is QSL's inference and is not stated as one. (2) "with QSpec's named code": QSpec has no unknown-node-kind code. The named-code rule is the owner ruling recorded at ADR-012:848 and :952. Fix: cite the ruling. Mark the profile sentence as an inference, or file it with the SC-G items. | spec/decisions/ADR-012-semantic-family-extension-contracts.md:1704-1716; spec/decisions/ADR-012-semantic-family-extension-contracts.md:848 |
| FND-012 | low | SC-Q1 is correctly left open, with two gaps. First, ADR-013's Status lets a "#209, #210, #222 or #229 decision" reopen a cell. #221 is not in that list, so "a #210-line decision" needs the ADR-013 owner to accept that reading. Second, parts of §16 depend on the answer and are not marked: §16.4 S6a admission, SC-R3's key check (FND-003), the §16.7 `quire-exact` row, and whether unions enter `TypeEnvironment.composites` (FND-001 item 6). Fix: list the dependents in SC-Q1, and state the ADR-013 Status basis as the owner's call. | spec/decisions/ADR-012-semantic-family-extension-contracts.md:1758; spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:23-24 |
| FND-013 | low | The §16.8 test plan is bounded but misses some §16.5 rows and cases: "duplicate member name in one union" (:1557); "stage limit or meter exhaustion" (:1558); the scrutinee-refused behaviour (FND-004); an unknown-member arm whose body is not checked (only indirectly, through TC-264 E03); and the parser rule that a scrutinee admits no top-level `record-value` (shared-grammar.md:405-408). SC-R3's foreign-key refusal half can be tested in process but is deferred with its wire half. Fix: add one case per item. | spec/decisions/ADR-012-semantic-family-extension-contracts.md:1557-1558; spec/decisions/ADR-012-semantic-family-extension-contracts.md:1676-1690 |
| FND-014 | low | "Scrutinee not a union" as `ill_typed`/`type-mismatch` is QSL's own choice; FR-146 is silent on it. It is also placed in `SumCaseCause`, while the identical (code, tag) construction refusals reuse `CheckCause::IllTyped`, so one (code, tag) pair has two cause paths. Separately, FR-146:72-74 reads literally as if an unknown member or wrong arity were an FR-143 construction refusal. TC-264 E03 and E04 settle it as `unknown-member` and `arm-arity`, and §16.5 follows the TC without citing it. Fix: reuse `CheckCause::IllTyped` for the scrutinee, or say why not. Cite TC-264 for the obligation reading. | spec/decisions/ADR-012-semantic-family-extension-contracts.md:1553; spec/decisions/ADR-012-semantic-family-extension-contracts.md:1568-1578; QSpec spec/functional/expressions/FR-146-check-total-pure-functions.md:72-74 |

## Verdict

Changes requested at c49eea84, with one medium item left: FND-015. FND-001
to FND-014 are verified fixed (see "Re-review (c49eea84)"). Fix FND-015 in
this PR. FND-016 and FND-017 are low and can go in the same pass. Once
FND-015 is fixed, the mapping meets the ticket's acceptance criteria.

Original verdict at c6e1e5ac: changes requested. The family assignment,
identity rows, S6a treatment and QSpec gap list were sound and measured. But
the ticket's first acceptance criterion was not met: "#187 can implement …
without inventing a new owner or bypass". FND-001 meant #187 would reopen
#221 on its first PR, FND-002 left the construction-dispatch owner
undecided, and FND-003 let untrusted input reach an `InternalFault`.

## Dispositions

Every finding is fixed in ADR-012 (and in the recipe for FND-007).

| Finding | Fix |
| --- | --- |
| FND-001 | §16.7 now allows four parts: the family and seam paths, compile-forced arms in the listed crates, tests, and the spec and docs. The Excluded list stays the hard boundary. It adds `cst.rs`, `qsl-forms/value.rs`, `checked_dispatch.rs` (arm binders are never renamed), `check/family.rs`, `check/check.rs`, and the extra composite-shape arms that follow if SC-Q1 is ruled (a). |
| FND-002 | §16.2 names one new `check`-core call-target seam function, which the `Call` and `Name` arms each call once. It states the resolution precedence, and a union name that clashes with an import alias refuses `ambiguous_declaration`/`ambiguous-name`. `Application` is unchanged. §16.6 now marks the enum and the function as new. |
| FND-003 | SC-R3 lists admission's four refusals and the payload reference walk, and says QSL admission owns them, not kernel `admits`. SC-Q1(a)'s value carries the union node key. §16.4's S6a row and §16.8's adverse-admission row match. |
| FND-004 | §16.5 "One refusal per `case`": typing comes first, a scrutinee refusal ends the builder, and `finish` reads arm heads only. §16.8 has a refusal-order row. |
| FND-005 | §16.5 "Result type": `case` uses the `if` rule (`Branches::Inferred`, `typing.rs:646-656`). The missing QSpec definition is filed as SC-G6. |
| FND-006 | Amended the §2 prose, the §11 #187 row, §12.1's Form and Witness rows and its tests paragraph, and §13.5's Q210-3. §16.4 no longer calls the Witness row "unchanged". |
| FND-007 | The recipe's "Required conversions" item 2 is keyed on families whose declarations S6a calls or selects. It names `SumCase`'s evaluator conversion and fixes the stale `s6a.rs` path. |
| FND-008 | §16.4 has a collection row: a union element type refuses `ill_typed`/`operator-ineligible` through `type_refusal`, as an IEEE-bearing element type does, and the kernel key arm yields no key (the O-13 population-pair precedent). §16.8 has a collections row. |
| FND-009 | An admitted `case` lowers its arms in `U`'s declared member order, and the SC-G1 proposal carries that rule. SC-R2 asserts the `case` node id and the `package_id`. |
| FND-010 | Corrected every citation the finding lists. |
| FND-011 | §16.9 cites the owner ruling for the unknown-node-kind code, marks the profile sentence as an inference, and files it as SC-G7. |
| FND-012 | SC-Q1 lists the parts of §16 that depend on it and leaves the ADR-013 Status basis to the owner. |
| FND-013 | §16.8 adds the missing rows: declaration duplicate member, limits, parse scrutinee, unknown-member body not checked, and admission. |
| FND-014 | The scrutinee refusal reuses `CheckCause::IllTyped`, and `SumCaseCause` holds only the exhaustiveness refusal. The obligation reading cites TC-264 E03 and E04. |

## Re-review (c49eea84)

Scope: `git diff c6e1e5ac c49eea84` (ADR-012 and
`docs/family-migration-recipe.md`). Each fix was measured against the tree
at c49eea84 and QSpec `origin/main`, not taken from the dispositions table.

Measured for the new text:
- `typing.rs:646-656` is the `Expression::If` arm calling
  `Self::conditional(Branches::Inferred(hint), …)`.
- `declaration.rs:1055-1060` is `type_refusal`'s collection rule. It refuses
  `operator-ineligible` for a non-`Sequence` whose element `contains_ieee`,
  and that walk descends through record and tuple declarations. A union rule
  there applies to every declared type, because `check_type`
  (`declaration.rs:974`) covers parameter and result types and :1099 covers
  declarations. Collection literals take their element type from that
  declared hint (`typing.rs:811-812`).
- ADR-013 O-13's Population row (ADR-013:345) says `compare_keys` "yields no
  key" for a population pair. The key-arm precedent is real.
- `ambiguous_declaration`/`ambiguous-name` is in the QSpec catalog
  (native-diagnostics.md:81) and in QSL (`refusal.rs:873`, `:908`).
- ADR-012:850, the v2 reader rule in §9, and ADR-013 QC-19 (:1135) back the
  §16.9 rewrite.
- `family.rs:795` and `:739`, `typing.rs:637` and `check.rs:1367` match the
  §16.2 description of today's resolution.
- The recipe edit keeps all three FR-066-AC-2 categories. No test reads the
  recipe text.
- The stale sentences that FND-006 listed are gone. The only remaining
  "variant declaration" (ADR-012:946) is covered by §12.1's amendment note
  ("the keyword is `union`").

| ID | Status | Note |
| --- | --- | --- |
| FND-001 | verified | §16.7 parts 1 to 4 cover every path the finding named: `cst.rs`, `qsl-forms/value.rs`, `checked_dispatch.rs`, `check/family.rs`, `check/check.rs`, tests, spec, and the SC-Q1(a) composite-shape arms. The Excluded list is unchanged. The host file of the new seam function is left as "the `check` core module that hosts `Typer`'s dispatch" (see FND-017). |
| FND-002 | verified, with a new defect | §16.2 now says no resolver exists today. It names one check-core call-target seam function, called once from each arm, with no branch in the arm, and `Application` is unchanged. The precedence rule rests on a false premise (FND-015). |
| FND-003 | verified | SC-R3 lists the four admission refusals and the payload reference walk, and puts them in QSL admission rather than kernel `admits`. The SC-Q1(a) value carries the union key. The §16.4 S6a row says an `InternalFault` can only come from a QSL defect. §16.8 has an adverse-admission row. |
| FND-004 | verified | §16.5 "One refusal per `case`" matches the fail-fast `Typer` and the one-refusal contract (`typing.rs:613-619`, `contract.rs:370-373`). A scrutinee refusal ends the builder, and `finish` reads arm heads only, which is consistent with §4.2. Typing first agrees with FR-146:264. The refusal-order tests are present. |
| FND-005 | verified | The `if` rule is cited correctly (`typing.rs:646-656`). SC-G6 is recorded as unfiled QSpec work. |
| FND-006 | verified | ADR-012 :270, the §11 #187 row, §12.1's Form, Witness and tests rows, §13.5 Q210-3, and the "unchanged" in the §16.4 Witness row are all amended. |
| FND-007 | verified | Recipe item 2 is keyed on "families whose declarations S6a calls or selects" and names `SumCase`'s `Machine::apply` conversion. The path is corrected to `s6a/mod.rs`. |
| FND-008 | verified | The collection row's path, precedent and tests hold (see above). The kernel has no map kind: `CollectionKind` is Sequence, Set, Bag and OrderedSet, so there is no map-key case. |
| FND-009 | verified | Arms lower in `U`'s declared member order. SC-R2 asserts the `case` node id and `package_id`, and the Identity test row covers arm reordering. |
| FND-010 | verified | All seven corrections are right. The FR-143 and FR-146 titles and sections, PR #121, 15 entries, the `S6aFamilyKind::family` wording, `identity.rs:538`, `emit.rs:279` and FR-092 each match the source. |
| FND-011 | verified | §16.9 cites the §9 owner ruling and QC-19. The profile bullet is marked as QSL's inference and recorded as SC-G7. |
| FND-012 | verified | The SC-Q1 dependents are listed, and the ADR-013 Status basis is left to the owner. The wording of that cell is garbled (FND-017). |
| FND-013 | verified | Rows exist for the duplicate member name, limits, the parse scrutinee, the unknown-member body, admission and collections. The limit oracle does not match its row (FND-016). |
| FND-014 | verified | The scrutinee refusal reuses `CheckCause::IllTyped`, and `SumCaseCause` holds only `UnprovedExhaustiveness`. TC-264 E03 and E04 are cited. |
| FND-015 | new, medium | The §16.2 precedence rule claims "Unions and enums share the type-declaration namespace, so a union and an enum with one name are already a duplicate declaration". That is false. The scope index keeps a `Vec` per type name (`check.rs:572-574`, `index.types`), and so does the enum-member index (`check.rs:579-591`). Same-named types are admitted, and a use refuses as ambiguous only at that point (`type_form.rs:124-133`; `Typer::name` `check.rs:1396-1402`). So `q::m` can name a union member and an enum member at once, or members of two unions named `q`, and §16.2 resolves neither. The seam function would silently prefer one, depending on the order in which it checks. Fix: state that `q::m` refuses `ambiguous_declaration`/`ambiguous-name` whenever `q` names more than one of: a union, an enum, an import alias (including two unions). Drop the duplicate-declaration sentence and add one adverse test (a union and an enum sharing a name, with `q::m` naming a member of both). Refs: spec/decisions/ADR-012-semantic-family-extension-contracts.md:1470-1478; qsl-semantics/src/check/check.rs:572-591; qsl-semantics/src/check/type_form.rs:124-133. |
| FND-016 | new, low | The §16.5 limit row's oracle is "limit kind work budget" (ADR-012:1615). The new §16.8 Limits test (ADR-012:1767) drives the checking depth bound on the FR-062-AC-7 pattern, which yields a nesting-depth limit. The row and the test name different limit kinds. Fix: have the row name both limit kinds (nesting depth from the `Typer`'s depth bound, work budget from a family meter charge), or make the test match the row. |
| FND-017 | new, low | Two wording slips. (1) In the SC-Q1 recommendation cell (ADR-012:1854), "It reopens one O-14 cell ("…"), ADR-013's Status lets …" is a comma splice that joins two sentences, so it reads as garbled. (2) §16.7 (ADR-012:1719) names the host of the new seam function only as "the `check` core module that hosts `Typer`'s dispatch". Name the file (`check/check/typing.rs` or `check/check.rs`) so the change set is concrete. |

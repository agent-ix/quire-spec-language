---
id: SR-635
title: "Fix-round review of ADR-015 after SR-633"
type: SpecReview
analysis: failure-domain
scope: "QSL branch qsl-255-dep-design head 620ed0b4: git diff HEAD~2..HEAD (84319959, 620ed0b4), read against origin/main...HEAD at main e86524bd (includes PR 445); QSpec companion FR-323 read at qspec-dep-design head e60521d"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-015
    type: reviews
---
# SR-635: Fix-round review of ADR-015 after SR-633

## Summary

Review set: the base checklist and failure-domain analysis, over the last
two commits, read with the whole branch diff. The check was whether each
SR-633 finding (FND-001 to FND-011) and SR-631 FND-007 and FND-011 are
fixed. The review also looked for new defects.

All eleven SR-633 findings are fixed as their dispositions state:

- FND-001: D-5, FR-099 and FR-099-AC-5 apply the rule to every type node
  reachable from the signature. TC-446 step 5 covers `Set<R>`, a tuple
  holding `R`, and `g::R`.
- FND-002: D-1 runs the cycle check first, then the diamond check. It
  reuses a library only once that library's compile has completed.
  FR-099 mirrors the order.
- FND-003: D-1 and FR-099 name three closure-level refusals and give each
  one's locus: the dependency input, the cycle and the diamond. Every other
  library refusal is wrapped. D-4 rule 4 and FR-098 cover the transitive
  replay case. FR-099-AC-3 and TC-446 step 3 add a wrapped `missing_import`.
- FND-004 and FND-005: D-4 has seven rules, and each rule runs over all
  entries before the next one. The ordering rule and the `DependencyInput`
  rule run before any recompile. FR-098-AC-7 and TC-444 step 7 cover both.
- FND-006, SR-631 FND-007 and FND-011: FR-091's import paragraph,
  FR-091-AC-24, TC-405 step 4 and its status all match the design. So do
  ADR-011 §2.4 and OQ-5, FR-087 AC-11, AC-14 and the AC-13 status,
  FR-098's status, TC-379, and the TC-379 and TC-405 rows in `spec/tests.md`.
  They now use a bare-hex digest, stage `intake` and the import's
  identity string.
- FND-007 to FND-011: the field is renamed `requested`, the QC-18
  acyclicity is stated, and D-5 gives a `generated` occurrence, which
  agrees with FR-093's occurrence rule. The added digest spellings, the
  locus change and the note on arbitrary cycle digests are all present.

Codes: every code and cause named is present. `invalid_package`,
`missing_import`, `stale_dependency`, `ill_typed`, `invalid-digest` and
`invalid-identifier` are in `docs/native-error-codes.md`.
`definition-cycle`, `invalid-value`, `missing-selection`,
`revision-mismatch` and `byte-digest-mismatch` are in
`qsl-cst/src/diagnostic.rs`. `conflicting-definition` is in
`qsl-package/src/checked.rs:198-201` and
`qsl-semantics/src/value/definition.rs:893`. `operator-ineligible` is in
`qsl-semantics/src/model/refusal.rs:704`. `PackageIdMismatch` maps to
`stale_dependency` (`qsl-replay/src/execute.rs:153`), as FR-323 rule 3
requires.

D-4 against QSpec FR-323: wherever both repos state a rule, the codes are
the same. That holds for the ordering and unreached-entry rules
(`invalid_package`/`invalid-value` at `/package/dependencies`), the
recompile's `missing-selection`, `revision-mismatch` and
`byte-digest-mismatch`, `PackageIdMismatch`, and the entry `package_id`
mismatch. The order differs in one place (FND-001 below).

The new findings are one medium and four low. None undoes an SR-633 fix.

## Findings

| ID | Severity | Summary | Refs |
|----|----------|---------|------|
| FND-001 | medium | D-4 does not follow FR-323's order, and it claims FR-323's codes for rules FR-323 does not state. QSL rule 1 (each `sources` names exactly one source, else `NotASource`/`SourceCount`, code `invalid-request`) runs before rule 2 (ascending entries). QSpec FR-323:41-43 makes the ordering rule the first rule. Take a request with two swapped entries, one of which names two sources. QSL refuses `SourceCount` (`invalid-request`). FR-323 refuses `invalid_package`/`invalid-value`. D-4 and FR-098:66-68 also say "with the refusal codes QSpec FR-323 gives", but FR-323 gives no code for rule 1 or rule 3 (`DependencyInput`, `conflicting-definition`). **Fix:** in ADR-015 D-4, swap rules 1 and 2. The ordering rule reads only identities, so it can run first. Say "with QSpec FR-323's codes for the rules FR-323 states; the one-source rule and the dependency-input rule are QSL's own, and both run before the recompile, where FR-323 places no rule". In FR-098, move the ordering bullet above the one-source bullet and use the same wording. | ADR-015 D-4 rules 1-3; FR-098:58-78; QSpec FR-323:38-43 |
| FND-002 | low | ADR-015's Context still says, in the present tense, that E3 refuses every import: "Five questions were still open, so E3 refuses every `import`" (ADR-015:39-40). This is the last such line in spec/. `spec/tests.md:252` also still says "TC-379 is blocked", which contradicts the TC-379 row and FR-087's AC-13 status ("planned with FR-099"). **Fix:** at ADR-015:39, change "refuses" to "refused". At `spec/tests.md:251-252`, replace "and TC-379 is blocked" with "and TC-379 is planned with FR-099 and ADR-015 D-5 (QSL-255 part b)". | ADR-015:39-40; spec/tests.md:250-252; FR-087 Status AC-13 |
| FND-003 | low | D-5's transitive rule and its explanation admit different sets. The rule admits every signature whose reachable type nodes carry no `declaration`. That includes a model-owned node, which carries `ModelOwner` and a `null` `declaration` (ADR-013 O-04), such as `M::T` inside `Reference<M::T>`. The explanation lists only "builtin, bounded-domain and anonymous structural types over them". A function `f(r: Reference<M::T>)` is admitted by the rule but not by the list. **Fix:** in ADR-015 D-5 and in FR-099's E3 bullet, say which it is. Either admit model-owned nodes, which the importer holds under the same id because D-1 compiles the library against the same package input, and add them to the list. Or refuse any reachable node that has an owner, and add a `Reference<M::T>` case to FR-099-AC-5 and TC-446 step 5. | ADR-015 D-5; FR-099 Behavior (E3 bullet); ADR-013 O-04 |
| FND-004 | low | D-5 says "the importing graph holds each under the id the dependency gives it", but it does not say which signature type nodes enter the importing package. It could hold every type node reachable from `f`'s signature, or only the ones its own nodes reference, such as the call's `result_type`. Take `g::f(3)` where the unit never writes `Int[0, 9]`: the emitted bytes, and so the `package_id`, depend on whether that parameter type node is held and given a `generated` occurrence. **Fix:** in D-5, state that the importing graph holds exactly the type nodes that its own nodes reference, and no other node of the imported signature. Add that to FR-099-AC-5, with a call whose argument is a literal. | ADR-015 D-5; FR-093 occurrences (:84-87); FR-099-AC-5 |
| FND-005 | low | Since the FND-002 and FND-003 fixes, the result of the FR-099-AC-3 diamond fixture depends on import order, and the fixture does not fix that order. If the unit declares `test/geometry` v1 before `test/a`, step 2 refuses the diamond unwrapped. If it declares `test/a` first, `test/a`'s import of v2 reaches step 3 first. No v2 is supplied, so it refuses a wrapped `stale_dependency`/`revision-mismatch`. **Fix:** in FR-099-AC-3 and TC-446 step 3, say "the unit declaring the `test/geometry` version `1` import before the `test/a` import". | FR-099-AC-3; TC-446 step 3; ADR-015 D-1 steps 2-3 |

## Verdict

**Changes required, minor.** Every SR-633 finding, and SR-631 FND-007 and
FND-011, is fixed. FND-001 is a real disagreement with QSpec FR-323. For
one request with two faults, QSL and QSpec would give different codes. It
needs a rule swap and one sentence in ADR-015 and FR-098. FND-002 to
FND-005 are wording fixes and one test clarification, and can land in the
same commit. No re-review is needed after these fixes.

## Dispositions

All findings are fixed in the commit that follows this review.

- FND-001: D-4 and FR-098 put the ordering rule first and say that the
  one-source and dependency-input rules are QSL's own preconditions, run
  before the recompile, with FR-323's codes for the rules FR-323 states.
- FND-002: ADR-015's context is in the past tense, and tests.md says TC-379
  is planned with FR-099 and ADR-015 D-5.
- FND-003: a reachable `ModelOwner` node is refused too (D-5, FR-099,
  FR-099-AC-5, TC-446 step 5), matching QSpec FR-322.
- FND-004: D-5 and FR-099 say the importing graph holds exactly the type
  nodes its own nodes reference; FR-099-AC-5 and TC-446 add `g::f(3)`.
- FND-005: FR-099-AC-3 and TC-446 step 3 fix the import order of the
  diamond fixture.
- QSpec SR-636 FND-002 (QSL side): D-5 uses the transitive closure of the
  signature type nodes' `dependencies`.

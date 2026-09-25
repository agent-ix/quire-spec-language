---
id: SR-633
title: "Pair review of ADR-015 with QSpec PR 161 (compile and replay against dependencies)"
type: SpecReview
analysis: integrity
scope: "QSL PR 447 head 30fa18f3 (git diff origin/main...HEAD at main be338c34): ADR-015, FR-099, TC-446, and the amendments to FR-001, FR-027, FR-071, FR-098, TC-186, TC-444, ADR-011, ADR-013, spec/spec.md, spec/tests.md; reviewed as one design with QSpec PR 161 head 8bdf48a4 (FR-307, FR-322, FR-323, TC-227, TC-233, TC-277, native-diagnostics step 7, both v2 schemas); checked for id collisions against open QSL PRs 445 and 446"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-015
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-099
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: reviews
---
# SR-633: Pair review of ADR-015 with QSpec PR 161

## Summary

Review set: the base checklist, failure-domain and cross-repo integrity,
over the diff only. QSpec PR 161 was reviewed together with this PR. Its
findings are in QSpec `reviews/2026-09-25-qsl-255-pair-review.md` (SR-634).
This review follows SR-631, the author's self-review, and does not repeat
the findings SR-631 fixed.

What holds up:

- D-2 (import digest), D-3 (library identity) and the D-4 request shape
  agree with QSpec FR-307, FR-322 and FR-323. The refusal codes and causes
  D-1 names exist in the code: `ResolutionCause::DefinitionCycle` and
  `InvalidValue` are admitted for `invalid_package`
  (`qsl-semantics/src/complete/package.rs:797-801`), `conflicting-definition`
  is `LibraryCause::ConflictingDefinition` (`qsl-semantics/src/library/mod.rs:257`),
  and `HostCause::SelectionIdentity`, `SelectionVersion` and `SelectionDigest`
  map to `invalid_identifier` and `invalid-digest` (`qsl-cst/src/diagnostic.rs:178-179`).
- D-5 is stated as design, not as a roadmap. Accepting only functions whose
  signature types carry no `declaration` is a sound V1 scope once FND-001 is
  fixed. Putting `dependency_reference` into the application-node preimage
  is consistent with ADR-013 O-04 and O-07. Node ids stay content-addressed:
  the term's tag differs from `reference`, and `{package, node}` separates
  two dependencies' declarations. Ids are stable for a fixed dependency
  `package_id` and re-key on any change to that dependency, as the
  Consequences state. QSpec's `tc_233_a_dependency_reference_enters_the_referencing_node_id`
  passes (`cargo test --test checked_package_v2` on the QSpec branch: 43
  passed).
- `ReplayRefusal::DependencyIdentityMismatch` is reachable and is distinct
  from `Recompile(DependencyIdentityMismatch)`. The entry's `package_id` is
  not used to build the dependency input (D-4 step 2). If only that field
  changes, the recompile and the step 4 `package_id` check both pass, and
  step 5 refuses. That is FR-098-AC-6's second case. A changed source
  refuses inside the compile, against the import's recorded digest.
  The two variants therefore have different triggers and different
  comparands.
- Entry identity plus `sources` is enough to say which source was meant as
  which dependency. The entry's `identity` and `version` select the import
  it serves, and its `sources` supply the bytes. The entry's `package_id`
  is only a claim, checked in step 5.
- The style is clean. No roadmap wording and no unsupported alternatives
  appear in the added text. The one "Until" in the ADR-013 QC-27 row was
  already on main.
- Measured: `make check-index-completeness` passes. `quire validate --scope . 'spec/**/*.md'`
  reports no error in a changed file. Every structural error it reports
  (MP-001..006, the tests.md column asserts) is also on main.

ID collisions: TC-446 is not on origin/main, but open PR 446 (QSL-226) also
adds `TC-446-identical-repeat-registration-is-idempotent.md` and TC-447. The
coordinator has ruled that 447 keeps TC-446 and 446 renumbers its own, so
this is not a finding against 447. FR-099, ADR-015, FR-027-AC-10,
FR-071-AC-9 and FR-098-AC-7 collide with nothing in PR 445 or 446. No
QSL-251 PR is open. PR 445 and this PR both edit ADR-011 and FR-098, and
PR 446 and this PR both edit ADR-013 and `spec/tests.md`. Those are
textual rebase conflicts, not id clashes.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
|----|----------|---------|------|--------------|
| FND-001 | high | The D-5 admission rule is not transitive. It says the parameter and result types "carry no FR-322 `declaration`", but an anonymous structural type (`Set<R>`, a tuple holding `R`, a compound-unit quantity over a declared unit) carries no `declaration` itself and still references a declared node. Its node id then depends on the dependency's owner, not the same in every package. The importing graph cannot hold it "under the id the dependency gives it" without also holding `R`, which D-5 forbids. The claim "Their node ids are the same in every package" is false for these types. **Fix:** state the rule as "parameter and result types none of whose reachable type nodes carries a `declaration`" in ADR-015:226-233 and in the FR-099:101-104 bullet. Add an FR-099-AC-5 case and a TC-446 step 5 case: a function over `Set<R>`, or a tuple containing `R`, refuses `ill_typed`/`operator-ineligible`. The same fix is SR-634 FND-001 in QSpec FR-322. | ADR-015:226-233; FR-099:101-104, FR-099-AC-5 (:122); TC-446 step 5 | wrong-requirement |
| FND-002 | medium | D-1 step 0 says "An import equal to an earlier one reuses its library". Step 2 says a library reached again while its compile is in progress refuses `definition-cycle`. Take the unit importing A, A importing B, and B importing A with A's exact identity, version and digest. B's import of A is equal to the unit's earlier import, so step 0 says reuse, but A is still in progress. The precedence is not stated. **Fix:** in ADR-015:99-100 write "reuses its library once that library's compile has completed". State that the in-progress check (step 2) runs before step 0's reuse. Mirror both in FR-099:72-88. | ADR-015:96-100, 110-112; FR-099:72-88 | wrong-requirement |
| FND-003 | medium | Nesting of refusals raised inside a library compile is ambiguous. Step 2 wraps "a library whose compile refuses" as `CompileRefusal::Dependency { path, refusal }`. A cycle A→B→A is detected while compiling B, and a diamond (FR-099-AC-3's `test/a` importing `test/geometry` v2) is detected while compiling `test/a`. Both are therefore that library's own refusal, and wrapped. Yet FR-099-AC-3 and TC-446 step 3 expect a bare `invalid_package`/`definition-cycle` and `conflicting-definition`. The same question applies to a replay whose removed entry is a transitive dependency: D-4 step 3 and FR-098-AC-7 say "at the import it supplied", which is inside a library. **Fix:** state which refusals are closure-level and reported unwrapped at the top-level compile: `definition-cycle`, the step 0 diamond refusal and the dependency-input refusals. Give each one's locus. State that every other refusal inside a library is wrapped, and say so for the transitive replay case in D-4 step 3 and FR-098:66-74. | ADR-015:112-117, 191-195; FR-099:85-88, AC-3 (:120); FR-098-AC-7 (:135); TC-446 step 3 | missing-requirement |
| FND-004 | medium | Two `dependencies` entries with one identity have no named replay refusal, and the two repos disagree on it. D-4 step 2 builds the dependency input, whose D-1 rule refuses a repeated identity `invalid_package`/`conflicting-definition`. No `ReplayRefusal` variant carries that. QSpec FR-323:45-48 refuses entries "not in strictly ascending UTF-8 byte order", which covers a repeat, as `invalid_package`/`invalid-value` at `/package/dependencies`. Two entries whose sources share one authority and identity are also unmapped. **Fix:** in D-4 and FR-098, refuse an entry list that is not strictly ascending by identity (a repeat included) as `ReplayRefusal::DependencySelections` (`invalid-value` at `/package/dependencies`), before step 2. The order check then leaves step 5, where it is redundant. Name the variant for a same-owner source collision. Add both cases to FR-098-AC-7 and TC-444 step 7. | ADR-015:184-188, 198-207; FR-098:66-83, AC-7; QSpec FR-323:45-48 | missing-requirement |
| FND-005 | medium | The step 5 order ("in this order") does not say whether each rule runs over all entries before the next, or each entry through all rules first. Take entry 0 with a changed `package_id` and entry 1 with an unreached identity. Rule-major order refuses `DependencySelections`, and entry-major order refuses `DependencyIdentityMismatch`. **Fix:** say "each rule over all entries, in entry order, before the next rule". That is the convention native-diagnostics uses ("each cause over all entries before the next cause"). | ADR-015:198-207; FR-098:75-83 | wrong-requirement |
| FND-006 | medium | SR-631 FND-007 is deferred to the implementing PR. Once PR 445 lands, main will contradict this design until code arrives. FR-091-AC-24 and TC-405 step 4 (PR 445) refuse an unsupplied import at stage `assembly`, with digest `sha256:…`. FR-099-AC-3 says stage `intake`. Under D-2, `sha256:…` refuses earlier, at S1 with `invalid-digest`. The ADR-011 §2.4 amendment in PR 445, FR-087's AC-13 status and FR-098's status say E3 refuses every import. **Fix:** when this PR rebases onto 445 (both edit ADR-011 and FR-098, so it must), rewrite those lines in this PR. Point FR-091-AC-24 and TC-405 step 4 at a bare-hex digest and stage `intake`, or retire them in favour of FR-099-AC-3. Repoint ADR-011 §2.4, FR-087 AC-13, FR-098 status and TC-379 status at ADR-015 and FR-099. | PR 445: FR-091 "Unsupplied import", FR-091-AC-24, TC-405 step 4, ADR-011 §2.4; SR-631 FND-007 | wrong-requirement |
| FND-007 | low | FR-098's O-26 list describes the new `ReplayRefusal` variant as "a dependency whose recompiled `package_id` differs from its recorded one". That is the `Recompile(DependencyIdentityMismatch)` trigger, not this variant's. The variant's `recorded` field is the request entry's claim, while elsewhere "recorded" means the import's digest. `PackageIdMismatch` calls the same kind of value `requested` (`qsl-replay/src/execute.rs:93`). **Fix:** reword FR-098:108-111 to "a `dependencies` entry whose `package_id` differs from the recompiled closure's selection of its identity". Rename the field `requested` in ADR-015:203-206. | FR-098:106-111; ADR-015:203-206 | wrong-requirement |
| FND-008 | low | D-5 puts a `package_id` into a node-identity preimage. ADR-013 QC-18 says the preimage names "the node's owner (not `package_id`), so the preimage stays acyclic". The design is consistent, because the id is the dependency's, fixed before the importer compiles, and D-1 refuses cycles. But neither D-5 nor the Amendments list says so, and QC-18 is not amended. **Fix:** add one sentence to D-5, and a QC-18 line to Amendments: the preimage never holds the node's own package's id, and a `dependency_reference` holds a dependency's, which D-1's cycle refusal keeps acyclic. | ADR-015:235-244, 246-268; ADR-013 QC-18 | missing-requirement |
| FND-009 | low | "with occurrences of its own" (ADR-015:229-230) does not say which occurrence a declaration-free type node gets when the importing unit writes no source occurrence of it, such as the `Boolean` result type reached only through `g::f(y)`. FR-322 `NodeBase` requires at least one occurrence (O-07). **Fix:** state that such a node's occurrence is the calling expression's region, with the role and ordinal O-07 gives a result type. | ADR-015:229-230; ADR-013 O-07; FR-099-AC-5 | missing-requirement |
| FND-010 | low | QSL tests less than QSpec on the digest spelling, and names a different locus. QSpec FR-307-AC-6 tests uppercase hex, 63 and 65 characters, while FR-099-AC-2 and TC-446 step 2 test only the `sha256:` prefix. QSpec FR-307 locates `missing_import` at "the import's identity string", while FR-099 says "at the import". **Fix:** add the three spellings to FR-099-AC-2 and TC-446 step 2. Say "at the import's identity string" in FR-099:78 and AC-3. | FR-099:76-80, AC-2, AC-3; TC-446 steps 2-3; QSpec FR-307-AC-6 | correct-requirement-no-evidence |
| FND-011 | low | The FR-099-AC-3 and TC-446 step 3 cycle fixture, where `test/a` and `test/b` import each other, cannot carry true digests. Neither `package_id` exists until the other's does. The case works only because the in-progress check fires before any digest check. **Fix:** state in TC-446 step 3 that the cycle's import digests are arbitrary, and that the refusal precedes the step 3 digest comparison. | FR-099-AC-3; TC-446 step 3 | correct-requirement-no-evidence |

## Verdict

**Changes required.** FND-001 must be fixed together with SR-634 FND-001.
FND-002 to FND-006 are ordering and mapping gaps that an implementer would
otherwise have to decide. Fix them in this PR. FND-007 to FND-011 are small
wording and test additions, and can land in the same commit.

## Dispositions

All findings are fixed, FND-006 after rebasing onto `main` with PR #445.

- FND-001: D-5, FR-099 and FR-099-AC-5 require package-independence of
  every type node reachable from the signature; TC-446 step 5 adds
  `Set<R>`, a tuple holding `R` and `g::R`.
- FND-002: D-1 runs the cycle check first and reuses a library only once
  its compile has completed; FR-099 mirrors both.
- FND-003: D-1 and FR-099 name the closure-level refusals (dependency
  input, cycle, diamond), reported unwrapped with their loci; every other
  library refusal is wrapped. FR-099-AC-3 and TC-446 step 3 add a wrapped
  transitive `missing_import`; D-4 and FR-098 state the replay case.
- FND-004 and FND-005: D-4 is a seven-rule order, each rule over all
  entries before the next, with QSpec FR-323's codes. A non-ascending or
  repeated entry list refuses `DependencySelections` before the input is
  built; a same-owner pair refuses `DependencyInput`. FR-098-AC-7 and
  TC-444 step 7 cover both.
- FND-006: after the rebase, FR-091's "Unsupplied import" is replaced by a
  pointer to FR-099, FR-091-AC-24 and TC-405 step 4 use a bare-hex digest
  at stage `intake`, and ADR-011 §2.4, OQ-5, FR-087's AC-13 and AC-14 status,
  FR-098's status and TC-379's status point at ADR-015 and FR-099. This
  also closes SR-631 FND-007 and FND-011; FR-087-AC-11 names the D-2 and D-3
  exception.
- FND-007: FR-098's list names the entry mismatch, and the field is
  `requested`.
- FND-008: D-5 and ADR-013 QC-18 state the acyclicity.
- FND-009: D-5 gives such a type node a `generated` occurrence when the
  unit writes none (FR-093).
- FND-010: FR-099-AC-2 and TC-446 step 2 add the three spellings; the
  `missing_import` locus is the import's identity string.
- FND-011: TC-446 step 3 states the cycle digests are arbitrary.

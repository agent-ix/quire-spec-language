---
id: SR-12254
title: "Gap analysis of quire-spec-language PR #1140 (QSL-924) against FR-394"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@9a3d2ce04c6ffcba3cdbc09803bfc2ec22f1651c; PR #1140 diff origin/main...HEAD; FR-394-AC-1 to FR-394-AC-5 (TC-1088), qsl-replay/src/admitted.rs, qsl-replay/tests/admitted_context_facade.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-394
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1088
    type: reviews
---
# Gap analysis of quire-spec-language PR #1140 (QSL-924)

## Summary

Ticket: QSL-924. Ticket text was read as data, and its acceptance list was
re-measured against the code. Plan completion was not assessed.

FR-394 is a new requirement with five ACs. Each AC has one test, and each test
is tagged `#[trace("TC-1088", "FR-394-AC-n")]`. The FR is listed in
`spec/spec.md`, both in the relationships and in the table. TCs are not
indexed there, which is the repository's convention. Every AC is examined:

- **AC-1** (same bytes and `package_id`, every case including an unused one,
  in canonical order, with the spine's keys): covered. The oracle is an
  independent spine run. It walks the same nominal nodes, which is what the AC
  asks for.
- **AC-2** (found by wire id, `Unknown` for an unadmitted id, `Unknown`
  naming a missing case): covered, with exact-equality asserts.
- **AC-3** (`Foreign` naming the library's `package_id`, own enum found):
  covered for a direct dependency only. See FND-001 and FND-002.
- **AC-4** (`Package` for a malformed source and an owned library, `Resource`
  for an S1 limit reached and for an S1 limit above the reader): all four are
  covered. The `Package` asserts are weak (code review SR-12253 FND-005).
- **AC-5** (root only): tagged, but the test is vacuous (code review SR-12253
  FND-001).

The ticket's consumer, Kani/IR-705 reading the enum key table, is served by
`AdmittedContext::enums()`, `enum_declaration` and `enum_case`. No stub was
found, and no code in the diff lacks an owning requirement.

## Verdict

No major gap. The transitive closure walk is not tested (medium). Fix it in
this PR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Foreign classification over the transitive closure is not tested. FR-394 Behavior and the ticket say "dependency closure", but the AC-3 test has one direct dependency, so the worklist step `pending.extend(dependency.dependencies())` never runs. A walk that stopped at direct dependencies would report a transitive library's enum as `Unknown` and still pass. Add a test (unit imports A, A imports B, and B declares an enum) that asserts `Foreign` naming B's `package_id`. Add "direct or transitive" to FR-394-AC-3. | qsl-replay/src/admitted.rs:304-322; qsl-replay/tests/admitted_context_facade.rs:208-244; spec/functional/FR-394-expose-the-admitted-enum-key-table-through-the-replay-facade.md |
| FND-002 | low | The AC-3 unit imports `test/shades` but never uses `Shade`. So the consumer path, where the unit's emitted package names the foreign enum's id, is not exercised, and `enums().count() == 1` is never checked while the unit references the imported enum. Make a function of the unit take or return `s.Shade`, and keep both asserts. | qsl-replay/tests/admitted_context_facade.rs:225 |

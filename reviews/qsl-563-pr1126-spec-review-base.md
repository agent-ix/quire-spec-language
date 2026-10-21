---
id: SR-13620
title: 'Spec review of PR #1126: protocol refinement rows, base checklist'
type: SpecReview
analysis: base
scope: 'agent-ix/quire-spec-language@74e87f43e; git diff origin/main...74e87f43e (PR #1126, branch qsl-563-spec, base main 6eceb158b): ADR-015, FR-027, FR-098, FR-099, FR-135, FR-136, FR-146, FR-147, FR-214, FR-310, TC-444, TC-446, TC-540, TC-541, TC-551, TC-659, TC-889; read against origin/main code (qsl-semantics check, qsl-cst grammar, qsl-replay request, src/command/wire.rs) and specs (FR-071, FR-141 to FR-148, ADR-015, ADR-027) and QSpec main 8736706 (FR-322, FR-323, FR-375, FR-432, shared grammar); ticket QSL-563'
review_set: subset
date: '2026-10-21'
relationships:
- target: ix://agent-ix/quire-spec-language/ADR-015
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-027
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-098
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-099
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-135
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-136
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-146
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-147
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-214
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-310
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-444
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-446
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-540
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-541
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-551
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-659
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-889
  type: reviews
---

## Summary

Ticket: QSL-563. PR quire-spec-language#1126, head 74e87f43e (branch qsl-563-spec), base main 6eceb158b. The diff (`git diff origin/main...HEAD`) touches 17 spec files, +267/-27. It is spec only, so code-review and gap-analysis do not apply. This file covers the checklist, the claims checked against the merged code and specs, traceability, and the PR body. The integrity, failure-domain and EARS lenses are SR-13623, SR-13622 and SR-13621.

Gates on the head: `make check-spec-validation` exit 0, with no diagnostic on any touched file. `tools/check-index-completeness.sh` exit 0. Tooling resolved quire 0.36.2 (engine 0.50.2) and quoin 0.28.3.

Claims checked against the merged code and specs:
- Check order: `refinement::check_refinement` runs at qsl-semantics/src/check/mod.rs:1685, and the protocol loop starts at :1848. Any refinement refusal returns at :1711, before a protocol is checked. FR-136 'Order of the checks' targets this correctly.
- Exactly one `finish`: the qsl-cst `ProtocolClause` rule ends `run Control Finish }`, with one required `P::Finish` (grammar.rs). QSpec shared-grammar `protocol-clause` ends `'run', control, finish, '}'`. The claim is true.
- Grammar: QSpec main 8736706 FR-375 has `abstract-subject = ident, [ '::', member-name ]`. FR-310's five syntax refusals and their tokens match FR-375-AC-11 one for one. qsl-cst still has `abstract ident concrete ident`, which the PR's remaining work lists.
- Refusal codes: `conflicting-binding`, `missing-name` and `malformed-declaration` are in FR-272's lists. They mirror the sibling rules: two rows for one class give conflicting-binding (FR-432, ADR-027 PR-1); an unknown name gives missing-name and a clause on another model gives malformed-declaration (FR-375 'Protocol subject').
- Sort key: it is total and deterministic. Protocol members are graph nodes with keys (FR-383: role, channel, template, control). Only `fork`/`join` of one `parallel`, `duplicate`/`lose` of one channel, `cattempt`/`cend` of one template, `spawn`/`retire` of one role, and `activate`/`memory` on the clause share a declaration, and each such pair differs by its keyword. `memory` rows differ by kind, and the empty keyword sorts first. So the tuple is unique once coverage holds.
- Replay: `PackageIdMismatch` maps to `stale_dependency`/`content-mismatch` (qsl-replay execute.rs), as FR-323-AC-9 states.
- The PR body states all three owner-level rulings (one protocol per refinement, the protocol sort key, `refinement_internals` as a new public API surface). It also lists the judgment calls and the remaining code work. Its QSpec follow-up for FR-432 and FR-375 is accurate as far as it goes. SR-13623 FND-003 and FND-004 name two differences from QSpec (FR-323, FR-375) that it leaves out.
- The added text has no changelog, status or owner wording. Each new AC is in a TC scope line. No hash, digest or pin is used for tracking: the node-key digest is the emitted product's own ordering key.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| ADR-015 refinement internals | spec/decisions/ADR-015-compile-and-replay-against-dependencies.md | examined |
| FR-027 request members | spec/functional/FR-027-export-compiled-native-package.md | examined |
| FR-027 library member rule | spec/functional/FR-027-export-compiled-native-package.md | examined |
| FR-027-AC-10 | spec/functional/FR-027-export-compiled-native-package.md | examined |
| FR-071 package reference | spec/functional/FR-071-implement-typed-replay-request.md | examined |
| FR-098 read internals | spec/functional/FR-098-execute-a-replay-request.md | examined |
| FR-098-AC-20 | spec/functional/FR-098-execute-a-replay-request.md | examined |
| FR-099 input | spec/functional/FR-099-compile-against-supplied-libraries.md | examined |
| FR-099 carry | spec/functional/FR-099-compile-against-supplied-libraries.md | examined |
| FR-099 no refuse | spec/functional/FR-099-compile-against-supplied-libraries.md | examined |
| FR-099 library defects | spec/functional/FR-099-compile-against-supplied-libraries.md | examined |
| FR-099 own value | spec/functional/FR-099-compile-against-supplied-libraries.md | examined |
| FR-099 replay | spec/functional/FR-099-compile-against-supplied-libraries.md | examined |
| FR-099-AC-9 | spec/functional/FR-099-compile-against-supplied-libraries.md | examined |
| FR-135 concrete_protocol | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md | examined |
| FR-135 header | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md | examined |
| FR-135 order | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md | examined |
| FR-135 internal input | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md | examined |
| FR-135-AC-10 | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md | examined |
| FR-136 sort key | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| FR-136 StepRowRef | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| FR-136 select rows | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| FR-136 one protocol | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| FR-136 two protocols | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| FR-136 unknown protocol | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| FR-136 memory row | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| FR-136 coverage inputs | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| FR-136 order models | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| FR-136 order protocol | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| FR-136 order refused | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| FR-136 unused operation | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| FR-136-AC-4 | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| FR-136-AC-6 | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| FR-136-AC-7 | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| FR-136-AC-8 | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| FR-146 protocol order | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| FR-146-AC-5 | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| FR-147 input | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md | examined |
| FR-147 order | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md | examined |
| FR-147 library defects | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md | examined |
| FR-214 select | spec/functional/FR-214-require-an-explicit-refinement-row-for-every-protocol-step-class.md | examined |
| FR-214-AC-5 | spec/functional/FR-214-require-an-explicit-refinement-row-for-every-protocol-step-class.md | examined |
| FR-310 header | spec/functional/FR-310-build-the-s2-form-of-a-refinement-declaration.md | examined |
| FR-310 syntax refusals | spec/functional/FR-310-build-the-s2-form-of-a-refinement-declaration.md | examined |
| FR-310-AC-2 | spec/functional/FR-310-build-the-s2-form-of-a-refinement-declaration.md | examined |
| FR-310-AC-5 | spec/functional/FR-310-build-the-s2-form-of-a-refinement-declaration.md | examined |
| TC-444 | spec/test-cases/TC-444-the-replay-executor-recompiles-selects-calls-and-refuses.md | examined |
| TC-446 | spec/test-cases/TC-446-spine-compile-resolves-imports-against-supplied-libraries.md | examined |
| TC-540 | spec/test-cases/TC-540-s3-checks-a-refinement-declaration-s-subjects-and-state-mapping.md | examined |
| TC-541 | spec/test-cases/TC-541-s3-checks-a-refinement-s-step-rows.md | examined |
| TC-551 | spec/test-cases/TC-551-per-step-simulation-records-are-written-and-settled.md | examined |
| TC-659 | spec/test-cases/TC-659-protocol-refinement-step-rows.md | examined |
| TC-889 | spec/test-cases/TC-889-s2-builds-the-refinement-form.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-027 now contradicts itself. The new text says 'each library object may carry `refinement_internals`, a JSON value'. Two sentences later, unchanged, a `1-draft` request with 'a library object that has any member besides `identity` and `source` (a `version`, for one)' refuses `invalid-request`, and FR-027-AC-10 tests that refusal. The code does the same: `wire::Library` is `deny_unknown_fields` with only `identity` and `source` (src/command/wire.rs:49-55). A coder cannot satisfy both. Change the member rule and FR-027-AC-10 to 'any member besides `identity`, `source` and `refinement_internals`'. | spec/functional/FR-027-export-compiled-native-package.md:67-74; spec/functional/FR-027-export-compiled-native-package.md:76-78; spec/functional/FR-027-export-compiled-native-package.md:133; spec/functional/FR-099-compile-against-supplied-libraries.md:49-54 |
| FND-002 | medium | The new CLI request behaviour has no acceptance criterion. That behaviour is the top-level `refinement_internals`, the per-library member, the empty list when the member is absent, and `invalid-request` for a `0-draft` request that carries it. TC-446 step 9 tests all of it ('Request the `1-draft` export with the member at the top level and on a library object, and a `0-draft` request with the member'), but its only new scope AC is FR-099-AC-9, which says nothing about the CLI request. FR-027-AC-10 is unchanged. Add an FR-027 AC (or extend AC-10) and trace TC-446 to it. | spec/functional/FR-027-export-compiled-native-package.md:67-74; spec/test-cases/TC-446-spine-compile-resolves-imports-against-supplied-libraries.md:70-77; spec/functional/FR-099-compile-against-supplied-libraries.md:183; spec/functional/FR-027-export-compiled-native-package.md:133 |
| FND-003 | medium | FR-098 says the executor 'SHALL read the optional `refinement_internals` value of the package reference, at `/package/refinement_internals`, and of each `dependencies` entry'. The executor reads FR-071's typed `ReplayRequest`. FR-071 was not amended: its package reference carries `sources` and `dependencies`, each entry `{identity, package_id, sources}` and nothing more. The code matches FR-071 (qsl-replay/src/request.rs `DependencyEntry` and `DependencyEntryWire` hold identity, package_id and sources only). FR-071 also does not say whether the decoder refuses, ignores or keeps an unknown member, so the member has no place to live. Amend FR-071 with the member on the package reference and on each entry, carried as raw JSON and not validated at decode, with an AC, and cite it from FR-098. | spec/functional/FR-098-execute-a-replay-request.md:97-104; spec/functional/FR-071-implement-typed-replay-request.md:72-77; spec/functional/FR-098-execute-a-replay-request.md:451; spec/test-cases/TC-444-the-replay-executor-recompiles-selects-calls-and-refuses.md:65-67 |
| FND-004 | medium | The PR covers the input side of `refinement_internals` but not the output side. QSpec FR-375 says the list 'is recorded in the lock (FR-322)'. QSpec FR-322 defines the lock member: canonical order, absent when every list is empty, never `null` or `[]`, and checked by FR-322-AC-77. QSpec FR-323 says a replay request carries the member 'when and only when the corresponding lock has one'. No QSL requirement says the emitter writes that lock member. FR-098-AC-20's first case ('carries it at `/package/refinement_internals` and recompiles to the proved `package_id`') assumes a producer that can copy it from the lock. Add the lock-member statement to the FR that writes the lock, or cite an owning QSL FR, and list it under the PR's remaining work. | spec/decisions/ADR-015-compile-and-replay-against-dependencies.md:89-93; spec/functional/FR-099-compile-against-supplied-libraries.md:93-99; spec/functional/FR-098-execute-a-replay-request.md:97-104 |

## Verdict

Not mergeable as it stands. FND-001 is a contradiction inside FR-027 that the code enforces today. FND-003 adds a member to a request type that FR-071 does not allow. FND-002 and FND-004 are traceability and output-side gaps. All four are text fixes in this PR. The rest of the base checklist passes: IDs, TC scopes, index rows, the rulings in the PR body, and the claims about the code.

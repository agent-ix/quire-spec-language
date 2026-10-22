---
id: SR-15110
title: "integrity review of STD-258 in-process provider entry over the checked package (PR #1206, QSpec PR #357)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@48fc9a675a111d90eb03746556d435d7e5eaaa0b; spec/functional/FR-280-serve-the-driver-s-lower-generate-and-prove-operations.md, spec/test-cases/TC-761-qsl-engine-manifests-register-and-the-provider-entry-returns-analyze.md; agent-ix/quire-specification@203fa2efdba64c0fe18dacbf0659d49420439e2d; spec/functional/tooling/FR-300-expose-rust-lifecycle-api.md, spec/objects/interfaces/FR-331-backend-provider-envelope.md, spec/test-cases/TC-220-rust-lifecycle-api-contract.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-280
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-761
    type: reviews
---
# SR-15110: integrity review of STD-258 (PR #1206, QSpec PR #357)

## Summary

Ticket: STD-258. Diffs reviewed: QSL `git diff origin/main...48fc9a675`
(FR-280, TC-761) and QSpec `git diff origin/main...203fa2e` (FR-300, FR-331,
TC-220). Both PRs are based on `main`, one commit each.

Checks:

- Faithfulness to the ticket. The ticket asks to choose between `prove`
  taking a checked subject for the statistical engine and probabilistic
  packages emitting (QSpec FR-322 v2 nodes). Both halves take the lead
  default (checked subject) and label it a lead default in the PR bodies.
- Code (read only). `emit_checked` refuses a package whose
  `probabilistic_model().first_declaration()` is set (qsl-package/src/emit.rs:1246),
  which matches FR-185 "Package emission". EN-4 reads `&CheckedPackage`
  through `subject.simulation.package` (qsl-analyze/src/statistical/model.rs:129-163).
  `replay_sampled_witness` takes a `WitnessReplay` over the `ModelSystem`
  and recompiles nothing (qsl-replay/src/statistical/witness.rs:216). FR-281
  Inputs list `&CheckedPackage`, so the claim that `analyze` reads the
  checked package holds at the spec level; no single `analyze` or provider
  entry exists in code yet, as the PR body says.
- FR-280-AC-5 against FR-193-AC-1/AC-5 and FR-189-AC-1: the `2 ms` claim
  with seed 7 and `max_witnesses` 3 settles Rejected and keeps three
  witnesses that replay `Reproduced` over `Service`'s checked subject. The
  new AC is consistent with them (see FND-005 for the fixture).
- Ids. FR-280-AC-5, FR-300-AC-6, FR-331-AC-33, TC-220 LC-06/LC-07 and TC-761
  step 4 are new and unique. The other open QSpec PRs touching these files
  (#354, #59) add none of these ids.
- Stale text. A grep of `spec/` and `proposals/` outside `reviews/` finds
  ADR-029 still stating the `EmittedPackage` form (FND-001). QSpec has no
  remaining statement that `prove` takes only the emitted package.
- QSL and QSpec halves. They agree on the direction, but not on the
  in-process input set (FND-002) or on how the new `unsupported` case is
  recorded (FND-003).
- Wording describes behaviour. The PR bodies do not paraphrase review
  findings.
- Gates. QSL `make check-spec-validation` exits 0, with no diagnostic on
  FR-280 or TC-761. QSpec `quire validate --scope . spec/**/*.md` gives the
  same diagnostic set as origin/main (0a11d84) after line normalisation; the
  one FR-331 warning moved line. `cargo test --test checked_package_v2
  --test backend_provider_v1` passes, 126 and 23 tests. Tools: quire 0.36.2
  (engine 0.50.2), quoin 0.28.3, newer than the 0.36.1/0.50.1 and 0.28.1 in
  the rollout note.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | QSL ADR-029 still says `prove` takes the `EmittedPackage` (OP table) and fixes the PV-3 `Provider::run(package: &EmittedPackage, ...)` signature that "the QSL engines of OP-2" implement. FR-280 now says the entry takes the `CheckedPackage` and no emitted package, and QSpec FR-300 gives `prove` the checked package. Over a package S4 refuses there is no `EmittedPackage` to pass, so a PV-3 adapter as written cannot serve the EN-4 item FR-280-AC-5 requires. Fix: update ADR-029's `prove` row and PV-3 (the trait takes the checked package, with the emitted package when one exists). | spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md:322; spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md:449; spec/functional/FR-280-serve-the-driver-s-lower-generate-and-prove-operations.md:67-73 |
| FND-002 | high | The two halves give different in-process inputs. QSL FR-280 Inputs: the `CheckedPackage`, the subject binding (FR-120), the routed items with `deadline_ms`, limits and `&Cancel`, and no manifest. QSpec FR-331 "In-process providers": the `manifest`, `deadline_ms` and items with the checked package, and no subject binding. QSpec FR-300's `prove` row lists no subject binding either, so `prove` has no binding to hand the entry, while FR-280-AC-5 and FR-300-AC-6 compare against `analyze` "for the same ... binding". Fix: state in FR-331 and FR-300 where the binding comes from (a `prove` input, or derived from the item), and make the manifest member agree in both halves. | spec/functional/FR-280-serve-the-driver-s-lower-generate-and-prove-operations.md:67-73; quire-specification spec/objects/interfaces/FR-331-backend-provider-envelope.md:325-332; quire-specification spec/functional/tooling/FR-300-expose-rust-lifecycle-api.md:61 |
| FND-003 | medium | FR-300 adds an `unsupported` case: an item routed to a process provider over a package `package` refuses settles `unsupported`, naming the kind and backend, and no frame is written. FR-331's `results` member lists when `unsupported` is recorded (a tool probe; a Kani `Unavailable`) and does not list this case. Neither file says whether it carries a closed cause or a diagnostic. Fix: add the case to FR-331 `results` and say which form it takes. | quire-specification spec/functional/tooling/FR-300-expose-rust-lifecycle-api.md:75-81; quire-specification spec/objects/interfaces/FR-331-backend-provider-envelope.md:93 |
| FND-004 | low | FR-331's `request` property still says, with no condition, that the request holds "the checked package as `package` and `package_id`". The new in-process section, which says the in-process form has neither, is placed under `## Wire schema` although it says the form is never written as a frame. Fix: qualify the `request` property to the frame form and place the section outside the wire schema, or point to it from the property. | quire-specification spec/objects/interfaces/FR-331-backend-provider-envelope.md:88; quire-specification spec/objects/interfaces/FR-331-backend-provider-envelope.md:321 |
| FND-005 | low | FR-280-AC-5 uses `Service` (FR-185-AC-7), whose `d` is `Millis = Int[1, 12]`, with ADR-024 §7.2's `2 ms` claim, which is over `d: Quantity<ms>`. It does not name the statistical method, which an FR-331 statistical item needs. Two test authors could build different fixtures. FR-193-AC-5 already pairs them the same way. Fix: name the §7.2 `Service` (refused at S4 as in FR-185-AC-7) and the method. | spec/functional/FR-280-serve-the-driver-s-lower-generate-and-prove-operations.md:118 |

## Verdict

Not mergeable as is. There are two major findings: FND-001 (ADR-029 still
states the `EmittedPackage` provider signature) and FND-002 (the halves give
different in-process inputs, and `prove` has no subject binding to pass).
FND-003 to FND-005 are minor. The direction is faithful to the ticket and
consistent with FR-185, FR-189, FR-193, FR-281 and the code.

## Disposition pass 1

Disposition pass 1 at QSL 203b8c172 and QSpec 4248c9f08 (fix commits 203b8c172 and 4248c9f08, diffs only). Gates in detached worktrees: QSL `make check-spec-validation` exits 0 with no diagnostic on FR-280, TC-761 or ADR-029; QSpec `quire validate --scope . 'spec/**/*.md'` gives 86 fail lines (FR-331's three diagnostics are the ones on main); `cargo test --test checked_package_v2 --test backend_provider_v1` passes 126 and 23. QSL PR #1206 is CONFLICTING with main: FR-280-AC-4 (main renamed `BackendDescriptor::admit` to `descriptor_from_manifest`, #1204) beside the edited AC-5 row.

Disposition pass 1: FND-001 and FND-002, the two majors, are fixed in both halves, and FND-003 to FND-005 are fixed. New: FND-006 (medium, minor) and FND-007 to FND-009 (low, minor). No major finding remains. QSL PR #1206 needs a rebase over #1204 before it can merge.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | medium | FR-280 says the provider entry "shall take the checked package and no emitted package" and "takes no `EmittedPackage`", but ADR-029 PV-3 now gives `Provider::run` an `emitted: Option<&EmittedPackage>` parameter that "the QSL engines of OP-2" implement, and passes `Some` whenever `package` emitted one. The ADR prose says an in-process provider ignores it; FR-280 says the entry does not take it. Two implementers would write different signatures. Fix: say in FR-280 that the entry ignores `emitted` and reads no emitted package, or keep `emitted` off the in-process path (for example the process adapter alone receives it). | spec/functional/FR-280-serve-the-driver-s-lower-generate-and-prove-operations.md:52,70,97; spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md:446-471 |
| FND-007 | low | The new ADR-029 bullet "ADR-029 OP-1 and PV-3, amended after acceptance" sits under "Amendments this record proposes", which lists amendments to other records applied "once this record is accepted". ADR-029 records no acceptance status, and the OP-1 row and PV-3 were already edited in place, so the bullet is a change note, not a proposed amendment. Fix: drop the bullet, or record ADR-029's status and say the record was revised in place. | spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md:865-869 |
| FND-008 | low | ADR-029 PV-3 says the process adapter given `None` returns `unsupported` naming the capability kind and backend (QSpec FR-300). QSpec FR-331 and FR-300 now fix the diagnostic `unsupported_projection`/`unsupported-requested-capability` with a `detail` that also says the package is not emitted. The QSL half omits the diagnostic and the not-emitted detail. Fix: name the diagnostic and detail in PV-3, or cite FR-331 "In-process providers" for them. | spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md:468-471; quire-specification spec/objects/interfaces/FR-331-backend-provider-envelope.md:298-302 |
| FND-009 | low | FR-331's `request` property now points to ("In-process providers" above), but the property is at line 88 and the moved `## In-process providers` section is at line 272, below it. Fix: say "below". | quire-specification spec/objects/interfaces/FR-331-backend-provider-envelope.md:88,272 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | ADR-029 OP-1 `prove` row takes the `CheckedPackage` and, when `package` emits it, the `EmittedPackage`, plus the subject binding; PV-3 `Provider::run` takes `&CheckedPackage`, `Option<&EmittedPackage>` and `&SubjectBinding`, with prose that `emitted` is `None` for a package S4 refuses. See FND-006 for the residual wording mismatch with FR-280. |
| FND-002 | fixed | Both halves now list the same in-process inputs: FR-280 Inputs add the request's `manifest`; QSpec FR-331 "In-process providers" adds the subject binding (QSL FR-120); FR-300's `prove` row and FR-300-AC-6 add the subject binding. |
| FND-003 | fixed | FR-331 `results` adds the case; "In-process providers" fixes `unsupported` with diagnostic `unsupported_projection`/`unsupported-requested-capability`, which the Result entries table admits for `unsupported` (exactly one of cause and diagnostic); FR-300 and FR-300-AC-6 name it. |
| FND-004 | fixed | The `request` property is qualified to the frame form and the in-process form, and the section moved out of `## Wire schema` to its own `## In-process providers`. See FND-009 for the wrong direction in the new pointer. |
| FND-005 | fixed | FR-280-AC-5 names the claim as the `2 ms` claim of ADR-024 §7.2 over `Service` (the claim of FR-193-AC-1 and FR-193-AC-5) and the Okamoto method (FR-189-AC-1); FR-185-AC-1 already fixes §7.2's `Service` with `d` typed `Millis`, so the fixture is the one TC-628 uses. |

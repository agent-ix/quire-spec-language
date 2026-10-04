---
id: SR-1283
title: "Spec review of quire-spec-language A2: a library carries no version"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@e3c96833abd7e79ca31e84acedd8948253ad39e3; A2's own commits only, git diff ad18594412cb4cdd849104a7643491b231cd79ee..e3c96833a: spec/decisions/ADR-013, ADR-015; spec/functional/FR-027, FR-071, FR-087, FR-098, FR-099, FR-100; spec/test-cases/TC-186, TC-253, TC-282, TC-444, TC-446; and the text those edits must agree with (ADR-015 D-1, FR-071 remaining work, FR-027 Status, QSpec FR-307 and FR-323 at quire-specification origin/main 396493c)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-027
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-087
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-015
    type: reviews
---
# Spec review of quire-spec-language A2

## Summary

Ticket: QSL-395 (A2, L2). No PR yet. The spec edits drop the library
`version` from ADR-013's replay-request row, ADR-015 step 5 and D-4,
FR-071's `dependencies` entry, FR-087's `LibraryPackage`/`Selection` and
AC-11 exception, FR-098, FR-100, the TC-186/253/282/444/446 procedures, and
FR-099's remaining-work line. FR-027 and TC-446 gain the new refusal: a
library object with a `version` member refuses `invalid-request`.

Checked:

- QSL-internal consistency. No QSL artifact outside `spec/reviews/` still
  gives a library a version, an empty-version refusal or a library
  `revision-mismatch`. The surviving `revision-mismatch` text (FR-116,
  `qsl-replay/src/execute.rs:198`) is frame and clause node identity, which
  is not A2's scope. Domain-package and `model` declaration versions are
  untouched, as ruled.
- ADR-015 D-4 rules 5 to 7 are unchanged, and D-4's entry shape is now
  `{identity, package_id, sources}`, which matches QSpec FR-323 on
  origin/main.
- The FR-001 source labels. FR-071 and ADR-015 rule 3 already state the
  two-label target (QSpec STD-150), and FR-027 and FR-071 each carry a
  remaining-work line naming Linear QSL-381 for the code. A2 leaves the
  four-label code alone. That is consistent; see the answer to point (a) in
  the report.

## Verdict

Mergeable after FND-001 is recorded. FND-001 is medium: QSpec, which QSL
cites as the authority for the supplied-library shape, still gives a library
a version, and A2 deletes the only line that recorded the divergence.
FND-002 and FND-003 are low wording fixes.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | QSpec FR-307 on origin/main (396493c) still offers a supplied library "under its identity `L` and its version string" and has the conflicting-definition refusal retain "both supplied libraries' identity and version". QSL cites FR-307 for the supplied-library shape (ADR-015 D-1, the `SuppliedLibrary` doc), and FR-087-AC-11 says FR-307's criteria pass against this code. A2 deletes the version from the code and deletes FR-099's remaining-work line, which was the only record of the gap. No QSpec ticket is cited. Fix: name the QSpec change (an STD or FR-307 ticket) in FR-099 or ADR-015 Status, as FR-071 does for STD-150. | spec/decisions/ADR-015-compile-and-replay-against-dependencies.md:64-66; spec/functional/FR-099-compile-against-supplied-libraries.md:170-186; spec/functional/FR-087-typestate-and-cross-package-node-key.md:601 |
| FND-002 | low | FR-027's Description and AC-10 read "A `0-draft` request that carries a library, a library with an empty identity, and a library object with any member besides `identity` and `source` ... refuse". The sentence makes all three cases `0-draft` requests, but the empty-identity and `version` cases are `1-draft` requests (and the test runs them as `1-draft`). The Description also refuses any extra member, while AC-10 names only `version`. Split the `0-draft` case out, and state the Description's wider rule in AC-10 or narrow the Description. | spec/functional/FR-027-export-compiled-native-package.md:68-71, :121 |
| FND-003 | low | FR-087-AC-11 says "ADR-015 D-2 and D-3 reshape two of these relocated types" and then lists four (`ImportDeclaration`, `LibraryName`, `LibraryPackage`, `Selection`). The count is stale. | spec/functional/FR-087-typestate-and-cross-package-node-key.md:601 |

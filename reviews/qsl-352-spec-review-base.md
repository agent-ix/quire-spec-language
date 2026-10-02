---
id: SR-1191
title: "Spec review of quire-spec-language PR #588 commit 2ae1acf9 (QSL-352): no contract version in the replay package reference, CallSiteRefusal codes, E7 by content"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@6ffdca6fe18c174773ef1b36e335bcc4cd7a33f2; PR #588 commit 2ae1acf9 against main 2ece5712"
review_set: subset
---
# Spec review of quire-spec-language PR #588 commit 2ae1acf9 (QSL-352)

## Summary

Ticket: QSL-352 (spec half). Commit 2ae1acf9 drops the contract version from ADR-013 O-25's harness list, O-26's package reference, FR-070 and FR-071; restates ADR-011 E7's obligation identity by ADR-013 O-09's content; and adds `CallSiteRefusal::code`, a settlement rule and FR-121-AC-14 with TC-516 step 14. The E7 member list (clause node id, clause occurrence key, obligation kind, arguments as parameter node id and declared domain, span excluded, RFC 8785 through §2's one encoder) equals O-09 and QC-14 word for word, and keeps the occurrence-key collision note. The `code` mapping matches `ReplayRefusal::code` (qsl-replay/src/execute.rs:233): `Recompile` and `DependencyInput` delegate, `UnknownFunction`, `UnknownOperation` and `UnknownClause` give `MissingDeclaration`, `Fault` gives `RuntimeInvariant`; FR-121 lists exactly the nine variants in qsl-replay/src/call_site.rs. The settlement rule agrees with the rule recorded on QSL-352 (comment of 2026-10-01, from CG #214 R-Q1, untrusted ticket text used as the reference the brief named): own-input refusal before the backend run settles `declined`, a replay-setup refusal after a refutation settles `inconclusive`/`ReplayRefused` with its code, a fault settles `failed`. That comment also names `InvalidFunction` and `Name` variants, which do not exist in the code; FR-121 is right to list the real ones.

Examined:
- ADR-013 O-26 (examined)
- ADR-013 O-25 (examined)
- FR-070 (examined)
- FR-071 (examined)
- ADR-011 E7 (examined)
- FR-121 code rule (examined)
- FR-121 settlement rule (examined)
- FR-121-AC-14 (examined)
- TC-516 step 14 (examined)
- ADR-015 D-4 (context_only)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | O-26 now gives the package reference as `package_id`, source digests and `dependencies`, with no contract version, and cites ADR-015 D-4 for it. ADR-015 D-4, which #588 does not touch, still says "The replay request's package reference is QSpec FR-323's `{package_id, contract_version, sources, dependencies}`". The two ADRs now disagree on the request's members. QSpec FR-323 keeps `contract_version` on the wire, and QSL-352 drops it only from the in-process struct, so D-4 should say that: amend D-4 to `{package_id, sources, dependencies}` and note that FR-323's `contract_version` is a wire member the in-process request does not carry until a byte reader exists. | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:934 |
| FND-002 | medium | FR-121-AC-14 is new and not implemented (FR-121's own Status now says "Remaining work: `CallSiteRefusal::code` (FR-121-AC-14)"), and TC-516's scope grows to AC-14. But the TC-516 row in spec/tests.md still lists FR-121-AC-1 to FR-121-AC-13 with "Passed locally", and the FR-121 row in spec/spec.md still reads "Implemented ... TC-516 passed locally". The index drops the AC-14 trace and overstates its status. Add FR-121-AC-14 to the TC-516 row, mark step 14 planned, and add the remaining work to the FR-121 row. | spec/tests.md:268 |
| FND-003 | low | The settlement rule is CG's C-09 terminal mapping (ADR-013: "CG owns C-09"), not QSL behaviour. It has no SHALL, no QSL acceptance criterion verifies it, and it cites only QSpec FR-331. Cite ADR-013 C-09 as the owner, so CG's mapping and this rule have one source. | spec/functional/FR-121-locate-a-function-call-site-through-the-replay-facade.md:166-170 |

## Verdict

Three findings. FND-001 (medium): ADR-015 D-4 still gives the replay package reference as `{package_id, contract_version, sources, dependencies}`, so the sweep is incomplete. FND-002 (medium): the FR-121 and TC-516 index rows still report AC-1 to AC-13 implemented and passed, without AC-14. FND-003 (low): the settlement rule allocates to CG and should cite ADR-013 C-09. Not mergeable until FND-001 and FND-002 are fixed.

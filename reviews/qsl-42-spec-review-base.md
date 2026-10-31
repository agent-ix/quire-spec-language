---
id: SR-1950003
title: spec-review/base review of QSL-42
type: SpecReview
analysis: base
scope: agent-ix/quire-spec-language; QSL-42; tests/it/unbounded_declarations.rs; qsl-eval/tests/it/state_model_family.rs;
  FR-334; TC-844
review_set: subset
---

## Summary

Frozen candidate: 43e116962ffbe1b166b6c4402b7cfcd0f32ff67d. Ticket: QSL-42. No branch writes. No applicable AssuranceProfile exists under spec/. Review selection: subset (base and ears-conformance); no structural, object or dependency change requires another lens. Criterion-strength/Jev and the remaining unrelated optional lenses were not selected. This is a changed-surface review, not repository-wide assurance. FR-334-AC-4 is unchanged, unexecuted, and explicitly retained in QSL-1368 blocked by STD-610. CG negotiation/warning delivery belongs to FR-076 and integration; FR-335 permits fixture dispositions here. No SMT installation or downstream run is required for this delta.

## Verdict

**CONDITIONAL** — One affected test specification still requires the removed package identifier.

## Visibility

GitHub repository visibility is PUBLIC. The old sync-thread claim is not treated as instructions or proof of current outward comment sync: the top-level Linear ADR-014 comment 9207e1b1 is absent from GitHub issue 189 comments, while the old GitHub-authored proof-backend comment is present on both. Findings are posted only as new top-level Linear comments; no findings go to GitHub.

## Review Evidence

The package_id erratum matches the current QSpec FR-322 Properties verbatim and preserves every part of AC-4. Changed requirements are concrete and independently testable. The owning TC-844 procedure/results were examined and still name the obsolete package identifier. This is a directly affected cross-reference consistency issue, not a request to reopen unrelated old test prose. Existing Verification cells with TC IDs and unrelated older prose are outside delta remediation. No new edges, domain objects, assurance framework or production capability. Independent checks: cargo fmt --check exits 0; focused Clippy for root and qsl-eval integration targets with -D warnings exits 0; root unbounded_declarations 8/8 and qsl-eval state_model_family 12/12 pass. Logs: /tmp/qsl42-review-{root,eval,clippy,spec,deny}.log. Root linker emits the existing __eh_frame >16MB warning. cargo deny check exits 4 with 255 rejected license requirements: advisories/bans/sources pass, licenses fail. Cargo manifests, Cargo.lock and deny.toml are identical to origin/main, so this dependency gate failure is outside the three-file delta; it is not claimed green. Whole-workspace tests/matrix were not run per dispatch; aggregate verification belongs to the lead. Quire 0.37.1 engine 0.50.3 / quoin 0.29.1 validation exits 0 with duplicate-archetype/inverse-edge and inline-data-schema module warnings.

## Scope

```yaml
scope:
- id: FR-334-AC-1
  path: spec/functional/FR-334-key-collection-types-by-the-root-definitions-identity-preimage.md
  role: examined
  excerpt: 'A unit whose only difference from another is `s: Set<Integer>` against
    `s: Set<Integer>[0, 18446744073709551615]` emits a different structural node key
    for `s`''s type and a structurally unequal package value; the unbounded node has
    no `collection_bounds` member and the bounded node has one with `min` 0 and `max`
    18446744073709551615.'
- id: FR-334-AC-3
  path: spec/functional/FR-334-key-collection-types-by-the-root-definitions-identity-preimage.md
  role: examined
  excerpt: Compiling the same bounded unit twice gives the same structural node keys
    and structurally equal emitted package values.
- id: FR-334-Behavior-package
  path: spec/functional/FR-334-key-collection-types-by-the-root-definitions-identity-preimage.md
  role: examined
  excerpt: Emitted packages SHALL compare structurally under QSpec FR-322, so changing
    only a bound's presence or value changes the emitted package value.
- id: TC-844
  path: spec/test-cases/TC-844-key-collection-types-by-the-root-definitions-identity-preimage.md
  role: examined
  excerpt: 'Verify that bound presence and value enter collection and population node

    identity and `package_id`, and that identities are deterministic.'
- id: FR-334-AC-4
  path: spec/functional/FR-334-key-collection-types-by-the-root-definitions-identity-preimage.md
  role: context_only
  excerpt: 'Conformance: for each collection and population type vector in QSpec FR-322,
    read in place from the QSpec repository by the test, QSL''s node key equals the
    vector''s expected key; the vectors exercise collection and population types covered
    by FR-144-AC-9, AC-13 and FR-153-AC-9.'
bindings: []
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | TC-844 still requires package_id comparisons after FR-334 AC-1/AC-3 switch to structural package equality; update its Description, procedure step 1 and expected steps 1/3 to the same contract, leaving step 4 unchanged. | spec/test-cases/TC-844-key-collection-types-by-the-root-definitions-identity-preimage.md:13 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Fix commit 72b7121ec35333f2c5a75191470d5fcef91b6e81 aligns TC-844 Description, procedure 1 and expected results 1/3 with structural node/package equality. Procedure 4, expected result 4 and FR-334 including AC-4 are unchanged. |

## Disposition pass 1

**PASS for the reviewed partial delivery** — no open review findings remain. Reviewed only the documentation fix and verbatim review-artifact custody; no second full review or broad test run. The lead retains aggregate gates and merge ordering (853, then 854, then 42). AC-4 remains unexecuted in QSL-1368 blocked by STD-610. This does not claim full assurance completion.

```yaml
dispositions:
- fnd: FND-001
  outcome: fixed
  after_excerpt: 'Verify that bound presence and value enter collection and population
    node

    identity and the emitted package value, and that structural identities and

    package values are deterministic.'
```

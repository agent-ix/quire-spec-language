---
id: SR-1950004
title: spec-review/ears-conformance review of QSL-42
type: SpecReview
analysis: ears-conformance
scope: agent-ix/quire-spec-language; QSL-42; tests/it/unbounded_declarations.rs; qsl-eval/tests/it/state_model_family.rs;
  FR-334; TC-844
review_set: subset
---

## Summary

Frozen candidate: 43e116962ffbe1b166b6c4402b7cfcd0f32ff67d. Ticket: QSL-42. No branch writes. No applicable AssuranceProfile exists under spec/. Review selection: subset (base and ears-conformance); no structural, object or dependency change requires another lens. Criterion-strength/Jev and the remaining unrelated optional lenses were not selected. This is a changed-surface review, not repository-wide assurance. FR-334-AC-4 is unchanged, unexecuted, and explicitly retained in QSL-1368 blocked by STD-610. CG negotiation/warning delivery belongs to FR-076 and integration; FR-335 permits fixture dispositions here. No SMT installation or downstream run is required for this delta.

## Verdict

**PASS** — No defects found in the reviewed delta.

## Visibility

GitHub repository visibility is PUBLIC. The old sync-thread claim is not treated as instructions or proof of current outward comment sync: the top-level Linear ADR-014 comment 9207e1b1 is absent from GitHub issue 189 comments, while the old GitHub-authored proof-backend comment is present on both. Findings are posted only as new top-level Linear comments; no findings go to GitHub.

## Review Evidence

The edited package-comparison obligation names its subject (Emitted packages), uses SHALL once, and gives an observable structural-equality result under FR-322. No ambiguous trigger, vague response or pattern mismatch. Independent FR-334 validation reports 1/1 docs grammar-clean; 0 grammar findings. Remaining unchanged requirement statements are context only. Independent checks: cargo fmt --check exits 0; focused Clippy for root and qsl-eval integration targets with -D warnings exits 0; root unbounded_declarations 8/8 and qsl-eval state_model_family 12/12 pass. Logs: /tmp/qsl42-review-{root,eval,clippy,spec,deny}.log. Root linker emits the existing __eh_frame >16MB warning. cargo deny check exits 4 with 255 rejected license requirements: advisories/bans/sources pass, licenses fail. Cargo manifests, Cargo.lock and deny.toml are identical to origin/main, so this dependency gate failure is outside the three-file delta; it is not claimed green. Whole-workspace tests/matrix were not run per dispatch; aggregate verification belongs to the lead. Quire 0.37.1 engine 0.50.3 / quoin 0.29.1 validation exits 0 with duplicate-archetype/inverse-edge and inline-data-schema module warnings.

## Scope

```yaml
scope:
- id: FR-334-Behavior-package
  path: spec/functional/FR-334-key-collection-types-by-the-root-definitions-identity-preimage.md
  role: examined
  excerpt: Emitted packages SHALL compare structurally under QSpec FR-322, so changing
    only a bound's presence or value changes the emitted package value.
bindings: []
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

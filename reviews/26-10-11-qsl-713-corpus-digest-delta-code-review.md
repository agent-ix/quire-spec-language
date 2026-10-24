---
id: SR-4955
title: "Code review of QSL-713 corpus digest test adaptation"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@1030aa0767d3218702b64aab4580e998ea3e5d9b against 6c14531b5e8bee29631cf97e035c3193ab080de8; qsl-semantics/tests/it/model_intake.rs; supporting reads: qsl-semantics/tests/it/identity_golden_vectors.rs, qsl-semantics/tests/it/main.rs, qsl-semantics/src/model/intake.rs, spec/functional/FR-260-admit-semantic-ir-documents-at-any-depth.md"
review_set: subset
---

## Summary

Independent static review of the tests-only corpus digest change in PR #749. The external FCD output uses its current canonical digest, while an unchanged QSL-owned golden vector retains an independent digest oracle.

## Verdict

**PASS** within this narrow delta; no findings. This review does not claim full PR qualification.

## Evidence

- `model_intake.rs:147` derives every offered digest through `PackageDocument::jcs_digest()`. `PackageDocument::parse` computes that value with `quire_canonical::sha256` at `intake.rs:351`; the test introduces no replacement algorithm or pinned FCD output digest.
- All three corpus members, the real FCD lift, dependency-resolved fixtures, 512 KiB stack, selection identity/version, digest domain, and real `admit` call remain. The positive assertion still detects refusal of any corpus package. It is a compatibility check, not an independent digest oracle.
- `identity_golden_vectors.rs:362` remains enabled by `tests/it/main.rs:14`. Its QSL-owned canonical text and literal expected digest independently constrain admission: the whitespace/order/escaped-Unicode input admits under that digest and refuses under its raw-byte digest. The adjacent RFC 8785 numeric-spelling control remains unchanged.
- The unchanged `lifts_the_architecture_bundle_and_admits_it` test still compares lifted output with the dependency's own expected document and checks admitted identity, version, and document content. No corpus bytes, golden vector, refusal assertion, or source implementation changed.

## Findings

No findings.

## Validation

`git diff --check` passed for the reviewed delta. No builds or test binaries were run; this is static review only. No applicable AssuranceProfile was found under `spec/`.

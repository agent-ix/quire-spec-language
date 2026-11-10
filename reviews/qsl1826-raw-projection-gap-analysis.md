---
id: SR-20261110034835820002
title: QSL-1826 legacy raw projection gap-analysis
type: SpecReview
analysis: gap-analysis
scope: agent-ix/quire-spec-language; QSL-1826 PR2644; 14 owner/test/fixture files
  plus SR77/78 factual custody
review_set: subset
---

## Summary

Frozen head 7e8d90da2afc4a3e28342113ee64db6717c9e055 against exact base bee3b8993ef8e75e099178256c86a179ef3acd60; ticket QSL-1826, PR2644. Read-only independent review of all 16 changed files. This is the existing legacy encoder/immutable-view foundation only. No bounded backend, caller-quota, cancellation, internal expansion policy, or reward/cyclic qualification is claimed. IR spec142/code143 and subsequent adoption remain separate dependencies.

The private projection owns value/composite/option/collection/union/enum/unit/reference and model/population/object/extents/envelope shapes. The retained sink preserves scalar rendering and legacy allocation behavior. Canonical byte sorting and map key-only stable ordering retain repeated pairs; cached names remain hints, not semantic tie-breakers. Borrowed metadata references validated fields directly; owned getters remain. reference_components follows existing ordered views and reverse identity index; identity_view and legacy identity share one validated model-node recipe. Cached model fields preserve original order after exact owner-plus-field clock filtering, then append existing clock shells once. Unsupported value branches and malformed record handling retain their previous distinctions. Existing caches and their hits/misses remain owned by KeyNames/Canonicalizer.

No manifests, lock, CI, replay, analyzer or IR changes occur. Peer rebase overlap is foundation diagnostic source_refusal borrowing and external checker/driver moves, with no changed encoder interface or caller overlap found. The 14 owner/test/fixture paths are byte-identical to tested 6213d635a33eb663827318d59a8b4af397397974. No concrete current-head compile risk warrants a new heavy batch. No post-rebase compile is claimed.

Accepted existing evidence inspected: encode unit 18 passed (/tmp/qsl1826-owner-model-encode-tests2.log), actual raw model golden 1 passed (/tmp/qsl1826-owner-envelope-goldens.log), bare and optional clock controls one each (/tmp/qsl1826-owner-bare-clock-key.log and /tmp/qsl1826-owner-option-clock-key.log), foundation literal/validation one passed (/tmp/qsl1826-owner-borrowed-identity-test.log). Golden capture provenance is pre-refactor, independently rechecked with old main encoder; expectations were not recaptured from this projection. Current diff whitespace check passed. No cargo, aggregate, Kani, full matrix, dependency or qualification gate was run by this reviewer.

Read reviewer, code-review, rust-review, rust-style and applicable API/implementation/safety/wire/testing references, repository README/AGENTS/CLAUDE and supplied owner handoff. No applicable AssuranceProfile exists. SR77/78 changes only restore factual residual default 16 MiB, leaving their original findings/dispositions/verdicts intact. No duplicate findings for that custody correction.

## Verdict

CONDITIONAL: manual scoped check found no new gap; whole-repository computed matrix and full acceptance qualification were excluded and are not asserted.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - | implementation-bug-despite-evidence |

## Coverage

Plan completion: not assessed. Manual scoped trace/behavior check only; computed full matrix explicitly excluded by dispatch. Optional semantic audit was not run. No spec/plan changes, so spec-review/sub-analyses do not apply. Trace bindings below are partial behavioral contributions, not proof of the entirety of each AC. In particular the small borrowed identity literal/validation test does not establish FR-259-AC-1's 100,000-term stack independence; this review records its narrower source behavior only. No new requirement coverage is claimed from a helper test alone.

## Examined scope and bindings

```yaml
scope:
- id: qsl-eval/src/simulation/model.rs
  path: qsl-eval/src/simulation/model.rs
  role: examined
  excerpt: '//! FR-120: [`ModelSystem`], the FR-101 [`TransitionSystem`] of a checked'
- id: qsl-eval/src/simulation/model/encode.rs
  path: qsl-eval/src/simulation/model/encode.rs
  role: examined
  excerpt: //! FR-120 "State, state key and anchor" and "Applications and transition
- id: qsl-eval/src/simulation/model/encode/projection.rs
  path: qsl-eval/src/simulation/model/encode/projection.rs
  role: examined
  excerpt: //! FR-120 / QSpec FR-181's borrowed typed-value projection, independent
    of
- id: qsl-eval/tests/fixtures/raw-model-key/legacy-enum.json
  path: qsl-eval/tests/fixtures/raw-model-key/legacy-enum.json
  role: examined
  excerpt: '{"bounds":{"entries":[],"type":"map"},"control":{"entries":[],"type":"map"},"memory":{"entries":[],"type":"map"},"observations":{"entries":[],"type":"map"},"queues":{"entries":[],"type":"map"},"roles":{"entries":[],"type":"map"},"semantic":{"entries":[{"key":{"type":"text","value":"{\"node\":\"ix://test/counters/counters\",\"package\":\"test/counters\"}"},"value":{"entries":[{"key":{"identity":"c1","object_type":{"original":{"node":"ix://test/counters/Counter","package":"test/counters"},"owner_effective_type":null},"type":"reference","universe":{"ordinal":0,"package":"test/counters","path":["m'
- id: qsl-eval/tests/fixtures/raw-model-key/legacy-extents.json
  path: qsl-eval/tests/fixtures/raw-model-key/legacy-extents.json
  role: examined
  excerpt: '{"bounds":{"entries":[],"type":"map"},"control":{"entries":[],"type":"map"},"memory":{"entries":[],"type":"map"},"observations":{"entries":[],"type":"map"},"queues":{"entries":[],"type":"map"},"roles":{"entries":[],"type":"map"},"semantic":{"entries":[{"key":{"type":"text","value":"relationships"},"value":{"entries":[{"key":{"type":"text","value":"{\"node\":\"ix://test/counters/relationship/Counter-link-Counter\",\"package\":\"test/counters\"}"},"value":{"complete":true,"instances":[{"source":{"identity":"c1","object_type":{"original":{"node":"ix://test/counters/Counter","package":"test/counte'
- id: qsl-eval/tests/fixtures/raw-model-key/legacy-unlinked.json
  path: qsl-eval/tests/fixtures/raw-model-key/legacy-unlinked.json
  role: examined
  excerpt: '{"bounds":{"entries":[],"type":"map"},"control":{"entries":[],"type":"map"},"memory":{"entries":[],"type":"map"},"observations":{"entries":[],"type":"map"},"queues":{"entries":[],"type":"map"},"roles":{"entries":[],"type":"map"},"semantic":{"entries":[{"key":{"type":"text","value":"{\"node\":\"ix://test/counters/counters\",\"package\":\"test/counters\"}"},"value":{"entries":[{"key":{"identity":"c1","object_type":{"original":{"node":"ix://test/counters/Counter","package":"test/counters"},"owner_effective_type":null},"type":"reference","universe":{"ordinal":0,"package":"test/counters","path":["m'
- id: qsl-eval/tests/it/model_simulation.rs
  path: qsl-eval/tests/it/model_simulation.rs
  role: examined
  excerpt: '//! FR-120 (TC-471, TC-472, TC-473): `ModelSystem`, the transition system
    of a'
- id: qsl-eval/tests/it/timed_subject.rs
  path: qsl-eval/tests/it/timed_subject.rs
  role: examined
  excerpt: "//! FR-231 (TC-686): the timed subject of ADR-026 \xA711's `Rpc` model,\
    \ its"
- id: qsl-foundation/src/node_wire.rs
  path: qsl-foundation/src/node_wire.rs
  role: examined
  excerpt: '//! The JSON shape of a structural node identity: `{"package": string,'
- id: qsl-semantics/src/check/check.rs
  path: qsl-semantics/src/check/check.rs
  role: examined
  excerpt: //! Name resolution and typing of Complete-V1 value expressions and function
- id: qsl-semantics/src/model/key.rs
  path: qsl-semantics/src/model/key.rs
  role: examined
  excerpt: //! FR-150 producer keys and structural model identities.
- id: qsl-semantics/src/model/normalize.rs
  path: qsl-semantics/src/model/normalize.rs
  role: examined
  excerpt: '//! FR-150 model normalization: phases 1 (decode), 2 (qualify), 3 (inherit),'
- id: qsl-semantics/src/model/observation.rs
  path: qsl-semantics/src/model/observation.rs
  role: examined
  excerpt: '//! FR-106: admit `quire.state.snapshot/v1` and `quire.state.invocation/v1`'
- id: qsl-semantics/src/model/state.rs
  path: qsl-semantics/src/model/state.rs
  role: examined
  excerpt: '//! FR-120: the layer-3 state-model seam a simulator of a checked package''s'
- id: reviews/qsl1826-state-refutation-implementation-code-review.md
  path: reviews/qsl1826-state-refutation-implementation-code-review.md
  role: examined
  excerpt: '---'
- id: reviews/qsl1826-state-refutation-implementation-gap-analysis.md
  path: reviews/qsl1826-state-refutation-implementation-gap-analysis.md
  role: examined
  excerpt: '---'
bindings:
- test_id: raw_model_key_preserves_legacy_reference_enum_and_extent_vectors
  ac_id: FR-120-AC-4
  trace: correct
- test_id: raw_model_key_preserves_legacy_reference_enum_and_extent_vectors
  ac_id: FR-120-AC-27
  trace: correct
- test_id: clock_shell_fields_match_exact_owner_and_declaration_keys
  ac_id: FR-120-AC-4
  trace: correct
- test_id: a_bare_clock_collection_ranges_its_initial_count
  ac_id: FR-120-AC-4
  trace: correct
- test_id: an_option_clock_has_a_presence_bit
  ac_id: FR-120-AC-4
  trace: correct
```

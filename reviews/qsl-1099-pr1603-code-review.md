---
id: SR-60029
title: "code review and rust review of QSL-1099 check_components and ComponentCertificate (quire-spec-language PR 1603)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@3414559bd129af5c60dfe512bdbbe7a353031f59 (reviewed code at 10e594c52, the PR's own commit; the head is a merge of main only); qsl-replay/src/certificate/{closure,recording,scc}.rs, qsl-replay/src/model_check/{item,settle}.rs, qsl-eval/src/simulation/{subject,protocol}.rs, qsl-analyze/src/model_check/liveness.rs and liveness/component.rs, qsl-analyze/tests/it/component_certificate.rs"
review_set: subset
date: '2026-10-26'
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-339
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: reviews
---
## Summary

Ticket: QSL-1099. Verdict: FAIL on one high finding (FND-001); the checker itself is sound against every certificate tamper I built.

Soundness probes (hand-built, run against the engine's certificate; none committed): merge two SCCs, merge waiting with accepting, split an SCC (trivial and weak tail), MissingAcceptance on an accepting component, out-of-range acceptance set, out-of-range constraint index, Trivial on a cycle, reversed order, no components, empty certificate, duplicated component (adjacent and appended), dropped last component, removed closure state, empty component, fabricated UnfairVisibility and UnfairStrong (empty, indexed, with sub), witnesses swapped between bindings, a foreign state in a component, a foreign package id. All are rejected with the expected rule (WitnessFails, BackwardEdge, NotPartition, InitialMissing) or PackageMismatch. A protocol (tso Spin) component certificate round-trips Accepted through check_protocol_components, and its reversed or emptied form is rejected.

Mutations (each reverted; run against qsl-replay certificate unit tests and the PR's component_certificate integration tests): backward edge off, duplicate owner off, uncovered state off, empty component off, MissingAcceptance disjointness off, visibility removed set not extended, UnfairWeak enabled-everywhere off, UnfairStrong taken check off, strong scope unfiltered, is_live ignoring removed, terminal stutter edge not recorded, enabled sets not recorded, acceptance membership not recorded, transition edges not recorded, package-id compare off (model): all KILLED. SURVIVED: the NotLiveness guard in check_system (FND-002) and the package-id compare in check_protocol (FND-002).

Gates: make ci exit 0 at 10e594c52 (33m45s). On the merged head the focused sets (qsl-analyze it: component_certificate, model_settle, liveness, analyze: 56 passed; qsl-replay lib: 878 passed) pass.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | A TP-4 item settles proved and certified over a plain ClosureCertificate, so a violated liveness claim can be certified. check_closure and check_protocol_closure run the closure only for a liveness instance (run_liveness checks BadState and nothing about acceptance), and settle_model_check and settle_protocol_check dispatch ProofCertificate::Closure to them for any item. Probe: the weak each clause with letters that never hold is Violated by the engine; growing a closure for it from the checker's own InitialMissing and SuccessorMissing rejections (36 states) gives Accepted, and settle_model_check over Holds{Closure} returns Proved{Exhaustive, Certified}. The same closure with the engine's components is rejected NotPartition. FR-126 says a TP-4 item SHALL NOT settle Holds without the ComponentCertificate FR-339's checker accepts; the checker does not enforce it, so a forged or buggy producer's Closure certificate passes. Fix: refuse (or reject) a Closure certificate for an item whose instances are Translation::Liveness in check_model and check_protocol, mirroring NotLiveness for components, with a test that settles a forged closure of a violated TP-4 claim and expects no proved. | qsl-replay/src/certificate/closure.rs (check_system), qsl-replay/src/model_check/settle.rs | implementation-bug-despite-evidence |
| FND-002 | medium | Two checker guards have no test that fails when they are removed. (a) The NotLiveness refusal in check_system (check_components over a non-TP-4 item). (b) The package-id compare in check_protocol: check_protocol_components is wholly unexercised by a committed test (FR-339-AC-7 is QSL-1060's), so its PackageMismatch-first rule, partition and witness rules over a protocol are untested; my probe shows it works. Add a foreign-package test and a NotLiveness test, and one protocol accept/tamper vector in this PR or an explicit QSL-1060 dependency. | qsl-replay/src/certificate/closure.rs, qsl-analyze/tests/it/component_certificate.rs | correct-requirement-no-evidence |
| FND-003 | low | Claimed residual not run, analysed: for a counted/interval protocol TP-4 clause the producer builds its automaton with translate (liveness.rs) while the checker's instances_for uses translate_counted for SubjectKind::Protocol, and product_refs keys the producer's states in the checker's key context. The certificate can therefore be rejected, or the producer faults (a-liveness-instance-has-a-liveness-translation) when counted progress lowers the clause to safety. The checker recomputes everything, so this is a completeness gap, never an unsound accept. A previously refused (unsupported) clause class now ends failed or CertificateRejected; give it a tracked ticket. | qsl-analyze/src/model_check/liveness.rs, qsl-replay/src/model_check/item.rs | correct-requirement-no-evidence |
| FND-004 | low | settle_model_check and settle_protocol_check doc comments still say only check_closure / check_protocol_closure check the certificate of a Holds outcome. closure.rs duplicates fairness is_live privately in scc.rs while qsl-eval's is_live is now pub for the producer only; harmless but a second copy. | qsl-replay/src/model_check/settle.rs, qsl-replay/src/certificate/scc.rs, qsl-eval/src/fairness/scc.rs | correct-requirement-no-evidence |

## Verdict

FAIL until FND-001 is fixed in this PR (the settlement of a TP-4 item must not accept a Closure certificate). No unsafe, no new #[allow], no dead code found in the diff; component work charges no budget and polls cancel as FR-339 specifies.

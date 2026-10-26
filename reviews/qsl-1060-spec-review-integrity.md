---
id: SR-1586002
title: "Integrity review of PR #1586 (QSL-1060 protocol-subject component certificate)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language; PR #1586 diff origin/main...qsl-1060-liveness-cert-spec; spec/functional/FR-228-derive-memory-fairness-constraints.md, spec/functional/FR-339-check-an-en-1-component-certificate.md, spec/test-cases/TC-526-an-en-1-component-certificate-is-checked.md, spec/test-cases/TC-673-memory-fairness.md"
review_set: subset
---

## Summary

Ticket: QSL-1060. A cross-artifact consistency check of the PR's FR-228 and FR-339 amendments against FR-126 "Certificates", FR-212, FR-338, ADR-025 and QSpec FR-430 and FR-436. It also checks the scope boundary with QSL-893 (slices QSL-1097 to QSL-1099). Five consistency defects were found: four medium and one low.

## Verdict

**Fix in this PR.** None of these defects is a wrong verdict. Each is a statement that a second implementer would read differently, or that contradicts another live requirement. Base review SR-1586001 FND-001 (high) already blocks merge.

On the scope boundary, the PR does not duplicate QSL-893's checker work. Its new producer rule and its References split do conflict with QSL-893's recorded scope (FND-005).

## Reviewed scope

- `FR-339` "Protocol subjects": examined. For a `flush(b)` member, the rule cites "FR-228's one class per thread of branch `b`". EN-1 SHALL list the resolved constraints in the same order when it writes a witness.
- `FR-339` Inputs and signature: examined. Over a protocol subject, the input is its `ProtocolCertificateRequest`. `check_components(request: &CertificateRequest<'_>, ...)`.
- `FR-339` References: examined. Model subjects belong to QSL-893 (QSL-1097 to QSL-1099), and protocol subjects to QSL-1060.
- `FR-228` Behavior: examined. A TP-4 item over a weak `parallel` settles `proved` only with an accepted `ComponentCertificate`, which carries `UnfairWeak` naming a flush constraint for each component that flush fairness alone starves. No request member, unit member or derived item SHALL remove a memory constraint from a fairness set.
- `FR-212` "Scheduler constraints" (with `scheduling adversarial`, the fairness set is the authored constraints alone), `FR-126` "Certificates" (first-holding witness order), `FR-338` (`check_closure` signature), QSpec FR-436 "Flush fairness" and QSpec FR-430: context_only.
- Linear QSL-893 (owner ruling and coverage comments) and QSL-1099: context_only. This is ticket text, read as data.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-339 "Protocol subjects" cites "FR-228's one class per thread of branch `b`" for a `flush(b)` member. FR-228 defines no such class. QSpec FR-436 adds one weak `each` member per thread, whose target is that thread's `flush` memory-step identity. FR-228-AC-1 spells the member by branch (`flush(left)`). Say that a `flush` member has one class, its thread's `flush` identity (QSpec FR-436), or define the per-branch reading in FR-228. | spec/functional/FR-339-check-an-en-1-component-certificate.md:99-101; spec/functional/FR-228-derive-memory-fairness-constraints.md:58-60, :65-68; QSpec spec/functional/memory/FR-436-explore-total-store-order.md:79-83 |
| FND-002 | medium | FR-228's new bullet requires `UnfairWeak` naming a flush constraint "for each component that flush fairness alone starves". FR-126 "Certificates" gives each component the first witness that holds, in the order `Trivial`, `MissingAcceptance`, `UnfairWeak`. A non-accepting component in which a buffer stays full gets `MissingAcceptance`, so the two statements contradict each other. Limit the FR-228 bullet to components where no earlier witness holds, or defer to FR-126's order. | spec/functional/FR-228-derive-memory-fairness-constraints.md:90-96; spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:477-483 |
| FND-003 | medium | FR-228's new bullet says no derived item SHALL remove a memory constraint, and that no `memory`-origin counterpart of `scheduling adversarial` exists. FR-212 still says that with `scheduling adversarial` the fairness set SHALL be the authored constraints alone, which removes the memory constraints. QSpec FR-430 says the adversarial set is the authored and memory constraints, and FR-228-AC-5's note relies on that. The FR-212 sentence is older, but this PR's new bullet now contradicts it. Amend FR-212's sentence and its Outputs bullet in this PR. | spec/functional/FR-228-derive-memory-fairness-constraints.md:100-106; spec/functional/FR-212-derive-scheduler-fairness-with-an-adversarial-opt-out.md:60-62, :99-100 |
| FND-004 | medium | FR-339's Inputs and AC-7 check the certificate under a `ProtocolCertificateRequest`, but `check_components` (like FR-338's `check_closure`) takes only `&CertificateRequest<'_>`. No entry point accepts the protocol request. Spell the protocol entry: an enum request, a second function, or a generic over the subject. | spec/functional/FR-339-check-an-en-1-component-certificate.md:49-51, :71-76, :207; spec/functional/FR-338-check-an-en-1-closure-certificate.md:66-72, :105-106 |
| FND-005 | medium | FR-339 now states a producer rule ("EN-1 SHALL list the resolved constraints in the same order when it writes a witness"), but FR-126 "Certificates", the producer FR, is unchanged. FR-126 still says `UnfairWeak` names "the first such constraint of the fairness set", not a resolved index. The new References line also splits ownership: QSL-893 for model subjects, QSL-1060 for protocol subjects. QSL-893's recorded scope differs. It is the single owner of `Holds{Exhaustive}`/`ComponentCertificate` for liveness and `check_components`, its coverage already lists FR-228's `UnfairVisibility` certificate, which is protocol-only, and QSL-1099 removes `ComponentCertificateNotBuilt` for every subject. Move the producer ordering into FR-126 "Certificates", and record the owner split on QSL-893 or QSL-1099 so the two lanes agree. | spec/functional/FR-339-check-an-en-1-component-certificate.md:115-117, :219-221; spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:477-485 |
| FND-006 | low | FR-339's new text cites FR-215, FR-221, FR-222 and ADR-025 (MS-4, MB-3, MF-4) with no `relationships` edge for any of them. FR-228's new SHALL makes FR-339 a hard dependency, but FR-228's frontmatter has no FR-339 edge (it is only in prose Dependencies). | spec/functional/FR-339-check-an-en-1-component-certificate.md:1-22; spec/functional/FR-228-derive-memory-fairness-constraints.md:1-27 |

## Dispositions

Round 1, against fix commit ff760ddb5 (`git diff c0f9ec111..ff760ddb5`).

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | FR-339 now defines the `flush(b)` member by QSpec FR-436: one member per thread, whose target names the thread path and whose one class is that thread's `flush` steps. A QSpec FR-436 edge is added. |
| FND-002 | fixed | FR-228 now defers to FR-126's first-holding witness order: `UnfairWeak` on a flush constraint applies only when no earlier witness covers the component. |
| FND-003 | fixed | FR-212's Outputs and its `scheduling adversarial` SHALL now keep the memory constraints and remove only the scheduler constraints. FR-212-AC-2 explains why its set stays empty, and FR-228-AC-1 and TC-673 step 1 add the `tso`-plus-adversarial vector. The governing ADR-027 PA-3 is still stale (new FND-007). |
| FND-004 | fixed | FR-338 adds `check_protocol_closure` and FR-339 adds `check_protocol_components`, both taking `&ProtocolCertificateRequest`. The FR-339 Behavior bullet and AC-7 and TC-526 step 6 use it. The name matches the existing `qsl-replay/src/certificate/closure.rs:330` function. |
| FND-005 | fixed | FR-126 "Certificates" now names FR-339's resolved-constraint order for `UnfairWeak`, and the producer sentence is gone from FR-339. The References line that split ownership between QSL-893 and QSL-1060 is removed, so the spec no longer contradicts QSL-893's recorded scope. |
| FND-006 | fixed | FR-339 gains edges to QSpec FR-436, FR-215, FR-221, FR-222 and ADR-025. FR-228 gains a `depends_on` edge to FR-339. |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | medium | ADR-027 PA-3, which FR-212 depends on, still says that with `scheduling adversarial` "the clause's fairness set is its authored constraints alone". Amended FR-212 now keeps the memory constraints, as QSpec FR-430 does, so FR-212 contradicts its own governing ADR. A reader of ADR-027 would drop the memory constraints. Amend PA-3 to "its authored and memory constraints" (ADR-025 MF-1) in this PR. | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:260; spec/functional/FR-212-derive-scheduler-fairness-with-an-adversarial-opt-out.md:100-102 |
| FND-008 | low | The new `check_protocol_closure` signature in FR-338 returns `TemporalCertificateCheck`. The function already in `qsl-replay/src/certificate/closure.rs:330` returns `Result<TemporalCertificateCheck, ClosureRefusal>`, as `check_closure` does at :276. The new spec signature copies the existing spec's omission of `ClosureRefusal`, and `check_protocol_components` will inherit it. Spell the `Result` and `ClosureRefusal` in FR-338 and FR-339. | spec/functional/FR-338-check-an-en-1-closure-certificate.md:105-117; spec/functional/FR-339-check-an-en-1-component-certificate.md:89-94; qsl-replay/src/certificate/closure.rs:276-281, :330-335 |
| FND-009 | low | Amended FR-212 now cites FR-228 for the memory constraints, but its frontmatter has no `relationships` edge to FR-228. | spec/functional/FR-212-derive-scheduler-fairness-with-an-adversarial-opt-out.md:1-27, :60-63 |

---
id: SR-1322
title: "Code review of quire-spec-language PR #642: BackendDescriptor carries ProviderOrigin"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@3ce34b67ac1653e193e7714dc652b5d229406429; PR #642 diff against origin/main (merge base a9cfe1113): qsl-route/src/lib.rs, qsl-route/tests/it/{main,provider_origin,route_registry,routing}.rs, tests/it/{lowering_registry_isolation,request_builder}.rs"
review_set: subset
---
## Summary

Ticket: QSL-637. PR: quire-spec-language#642. Code review with the rust-review
lane folded in. No build was run: `make ci` exit 0 on this head is the gate
(logs/qsl-637-make-ci.log).

- `ProviderOrigin { Linked, Process }` is a plain `Copy` enum in layer R,
  public at the `qsl_route` root, so CG reads it from the same crate it reads
  descriptors from. No manifest member feeds it: `admit` takes it as a
  separate argument beside the advertised pairs.
- `BackendDescriptor::new` and `admit` take `origin` as the second argument;
  `origin()` reads it. Every caller (qsl-route unit tests, the three
  qsl-route it modules, and two root-crate it tests) passes `Linked`.
- Conflict: `Registry::register` compares descriptors with the derived
  `PartialEq`, which now includes `origin`. So a Linked/Process pair of one
  identity takes the existing `Some(existing)` branch: both withdrawn,
  `duplicate-backend` per distinct digest (one here, since the manifest is the
  same), and the identity moves to `conflicts`. An identical registration
  hits `*existing == descriptor` and stays idempotent. Order independence
  follows from the symmetric equality and the `BTreeSet` of digests.
- No code infers `Process` from an identity: grep finds no constructor of
  `ProviderOrigin::Process` outside tests.
- No panic, unsafe, integer conversion or lock surface is added.

## Verdict

Approve. The code implements the ProviderOrigin amendment (option b) and the
Q6 conflict rule correctly. FND-001 is a comment-layout nit.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The edited `BackendDescriptor` doc comment has one over-long line ("... (FR-288-AC-6). The manifest digest only tells two registrations of one"), about 100 columns, where the rest of the block wraps at 78. Fix: reflow the paragraph. | qsl-route/src/lib.rs:203 |

## Dispositions

Round 1, reviewed at eb5ccaf2fbb77a4ff0f439931c6a7aa8be9da1f0 (fix commit eb5ccaf2f on 3ce34b67; `git diff 3ce34b67 eb5ccaf2`). No build run; the change is spec text plus one comment reflow. The round adds no new finding.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | eb5ccaf2f |

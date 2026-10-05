---
id: SR-1323
title: "Gap analysis of quire-spec-language PR #642: FR-288-AC-1, AC-5, AC-6 against TC-772 tests"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@3ce34b67ac1653e193e7714dc652b5d229406429; FR-288 Behavior and AC-1 to AC-6; TC-772; qsl-route/tests/it/provider_origin.rs; qsl-route/src/lib.rs (BackendDescriptor, Registry::register)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-288
    type: reviews
---
## Summary

Ticket: QSL-637. Manual AC-to-test check (no plan bundle).

- **FR-288-AC-6** — `origin_differing_registrations_conflict_in_either_order`
  registers Linked then Process and Process then Linked, and asserts one
  `duplicate-backend` refusal carrying the manifest digest, zero held
  descriptors, and `UnknownBackend` for a named `kani` lookup.
  `identical_origin_registration_is_idempotent` covers both origins. Binding
  correct; covers every clause.
- **FR-288-AC-5** — `descriptor_holds_the_origin_it_was_built_with` asserts
  `origin()` returns what `new` and `admit` were given, and that the id is
  unchanged. It cannot show what the AC states (see FND-001).
- **FR-288-AC-1 (reworded)** — "differ in their origin member alone" agrees
  with AC-5 and the code. No test carries the tag (FND-002).
- **FR-288 Behavior, new bullets** — the conflict bullet matches
  `Registry::register`. The origin bullet names a conversion that is not in
  this repository (FND-001).

## Verdict

Changes requested, spec text only. AC-6 is fully backed. AC-5 and the
origin Behavior bullet bind a conversion that the driver owns, so TC-772 can
only test storage; the AC should say whose code it binds.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-288-AC-5 says a descriptor "built for a compile-time provider holds origin Linked, and one built for a plugin hello manifest holds Process", and the Behavior bullet says "The conversion shall set Linked ... and Process". qsl-route has no manifest-to-descriptor conversion and no hello handling; origin is a caller-supplied argument to new/admit, and ADR-029 PV-1 says the driver builds the registry. So TC-772's test only proves the descriptor holds the origin it was given; the Provider-to-Linked and hello-to-Process choice is the driver's code (IR-609 lane) and no test here can fail if the driver gets it wrong. Fix: reword AC-5 to what layer R owns (the descriptor holds the origin passed to new/admit, and no manifest input reaches it), and add a sentence that the Linked/Process assignment at the Provider and hello call sites binds the driver's registry builder and is verified there. | spec/functional/FR-288-build-the-registry-from-provider-manifests.md:81 |
| FND-002 | low | FR-288-AC-1, as reworded, still describes passing one manifest "to the conversion" twice; that conversion does not exist in the repo and no test carries FR-288-AC-1 (nor AC-2 to AC-4), while spec/tests.md keeps TC-772 at Planned. Fix: when FND-001 is resolved, state AC-1 against the same owner, or leave it to the driver's conversion explicitly. | spec/functional/FR-288-build-the-registry-from-provider-manifests.md:77 |

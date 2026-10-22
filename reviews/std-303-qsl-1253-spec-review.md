---
id: SR-15830
title: "Spec review of quire-spec-language PR #1253 (STD-303): S4 emission of model terminal declarations"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@c0622e1aa3edc1e9c8b061422b213c795c46b310; PR #1253 diff against main 5c4a1da9; FR-411, TC-1142, FR-382 behaviour bullet and FR-382-AC-6, TC-1059 row 6, spec/spec.md; checked against QSL FR-124, FR-126-AC-9, FR-128, qsl-semantics/tests/it/terminal_declarations.rs and QSpec FR-476 at e7b95454"
review_set: subset
---
# Spec review of quire-spec-language PR #1253

## Summary

Ticket: STD-303. The PR adds FR-411, which covers S4 emission of a checked `TerminalDeclaration` as a QSpec FR-476 `temporal`/`terminal` node, I2 read-back, the subject-identity binding and replay reading the declaration from the package. It also adds TC-1142, rewrites the FR-382 behaviour bullet, FR-382-AC-6 and TC-1059 row 6 so that they no longer refuse `not-yet-implemented`, and adds a spec.md row.

Examined:
- FR-411 Description, Outputs, Behavior, FR-411-AC-1 to AC-4 (examined)
- TC-1142 procedure and expected results (examined)
- FR-382 S3 bullet, FR-382-AC-6, TC-1059 row 6 (examined)
- QSL FR-124 `self` rules, FR-126-AC-9, FR-128 envelope and Deadlock replay, FR-128-AC-6 (context_only)
- QSpec FR-476 body and reader order at #375 head (context_only)
- qsl-semantics/tests/it/terminal_declarations.rs FR-382-AC-6 test (context_only)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-411 tells the emitter to write `parameters` "as a reference to the `self` parameter node", and FR-411-AC-1 says "`parameters` the `self` parameter node". QSpec FR-476 makes `parameters` an aggregate of references. An emitter that follows FR-411 literally writes a bare reference, which FR-476's Shape step refuses. Write "an aggregate holding one reference to the `self` parameter node" in both places. | spec/functional/FR-411-emit-a-checked-model-terminal-declaration-as-a-v2-terminal-node.md:67-69; :93 |
| FND-002 | medium | "The request writer SHALL bind the model subject's terminal node id ... into the subject identity" names no place for the binding. The subject already carries the package through `package_id`, which replay checks by FR-098, and QSpec FR-368's `subject` member has no terminal field. A coder could add a request member that QSpec does not define. FR-411-AC-2's identity clause holds through `package_id` alone, so it cannot tell. State that the binding is the `package_id` (QSpec FR-361 and FR-366), or drop the bullet. | spec/functional/FR-411-emit-a-checked-model-terminal-declaration-as-a-v2-terminal-node.md:76-79; :94 |
| FND-003 | medium | FR-411-AC-3 quotes FR-128-AC-6's predicate as `terminal when 6 / (3 - c.value) = 0`. In a `terminal` declaration nothing binds `c`, so the unit as quoted does not check, and AC-3 now requires it to compile to a package. AC-3's "reads the declaration from the recompiled package" also gives the same outcome as reading the recompiled checked model, so the AC cannot fail on that claim. Name the full source, for example `terminal Done using v on Counter when { 6 / (3 - self.value) = 0 };`. Then either drop the "reads from the node" bullet or make it observable. | spec/functional/FR-411-emit-a-checked-model-terminal-declaration-as-a-v2-terminal-node.md:80-82; :95 |

## Verdict

The split is right. A separate emission FR sits beside FR-337, FR-124 keeps the S3 check, and the FR-382 bullet, FR-382-AC-6 and TC-1059 row 6 now state the emitted node with no compatibility shim and no status wording. FR-411 defers body, identity and refusals to FR-476 and repeats only what QSL writes and reads. AC-1, AC-2 and AC-4 are testable and can fail. AC-4's refusals are a valid subset of FR-476's. The ids FR-411 and TC-1142 are unique on main and across the 45 other open PRs. `QSPEC_DIR=<#375 head> make check-spec-validation` exits 0, with no warning on FR-411, TC-1142, FR-382 or TC-1059.

The test at qsl-semantics/tests/it/terminal_declarations.rs:167-187 is tagged FR-382-AC-6 and still asserts the old `not-yet-implemented` refusal. Once this merges, the follow-up code ticket must replace it. This PR is spec-only, so that is not a finding here. **Mergeable after #375 merges and FND-001 to FND-003 are fixed.**

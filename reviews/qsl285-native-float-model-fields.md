# QSL-285 independent spec review

Reviewed PR #715 at `c8dcbda09bfa6d8b9a2fa9d4468a05331e11bfe8` against base `ea3838315104ac3092222f1804517d5c315cb7df`. PR head is MERGEABLE. Static review only; no build run.

## Verdict: PASS

- The FR-056 delta adds AC-20 for both native Float32 and Float64 model fields, states the no-type outcome, refusal code/cause and locus, no `PackageDeclarations`, and explicitly avoids assigning a rounding mode. It preserves the existing FR-056-AC-16 text exactly; the one-line addition does not disturb #704's concurrent AC-16 work.
- The TC-405 delta cleanly separates contexts: function-signature floats retain FR-091-OQ-4 mode behavior; bare types remain `exact` per FR-148; model-owned native float fields remain an explicit unsupported-feature refusal. Its new step 5 covers each width, and its Status explicitly says the new refusal test has no implementation backing yet, so it claims no runtime evidence.
- Cross-check: QSpec FR-322 at merged `origin/main` `7a81a2f` lists Boolean, Integer, bounded Integer/Text, and same-package object references as model-owned member types; Float32/Float64 are absent. It says a declared type outside the table has no member type. QSL FR-091-OQ-4 independently says only domain-package model fields still get the floating-type error (`unknown_required_feature`/`unsupported-feature`), while signatures/bodies are admitted and bare floats are exact. The new wording is consistent with both contracts and does not imply that QSpec accepts the field.
- `git diff --check` passed. Focused `quire validate` of the two changed artifacts exited 0; output contained only existing module-discovery/archetype warnings.

## Finding

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-056-AC-20's Verification cell says `Test (TC-405)`, while the spec-review checklist says Verification cells name a method only and omit TC IDs. The surrounding FR-056 table uses the older `Test (TC-...)` convention, so this is a consistency/style issue rather than a conformance blocker. | FR-056 AC-20; quoin spec-review checklist |

No substantive semantic or coverage blocker found. TC-405 describes step 5 as unbacked rather than passing, which is appropriate for this spec-only change.

## Disposition on final head

Reviewed `425fb2f53313d011399bed3d8dcf85d49bffa3b7` (`c8dcbda09..425fb2f53`): FND-001 is fixed by the sole changed line, which now says `Test`. `git diff --check` passes. GitHub reports PR #715 head `425fb2f53313d011399bed3d8dcf85d49bffa3b7`, base `ea3838315104ac3092222f1804517d5c315cb7df`, and `MERGEABLE`. Final disposition: PASS; no open findings.

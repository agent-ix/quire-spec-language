

Round 3 (coordinator, from draco's review of #1210): the core-only basis list is six. `exact-value`, `value-bounds` and `zone-certified` have no `ProofBasis` variant in qsl-replay, so nothing to refuse in code; the basis match in `from_plugin_proof` is now exhaustive, so a variant added later forces a decision. FR-289-AC-10 and TC-773 list the six. Built against #1210 head 1892b87 (lists three bases). Fix commit SHA.

# IEEE 754-2019 binary profile definition

Definition identity: `quire.value.ieee754-2019-default/v1`; revision:
`1-draft.1`. This is a leaf semantic-profile definition selected alongside
`quire.value.complete/v1`; it has no reverse dependency on that root.

This definition selects FR-148's binary32/binary64 interchange formats, five
IEEE rounding directions, strict language-level `exact` policy, deterministic
leftmost-NaN quieting/payload rule, canonical invalid-operation NaNs, operation-
local exception flags, tininess-after-rounding rule, comparison relations and
qualified intrinsic identities. A host floating implementation is not the
authority for these choices.

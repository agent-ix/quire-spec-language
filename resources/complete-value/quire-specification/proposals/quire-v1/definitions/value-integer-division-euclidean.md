# Euclidean integer-division definition

Definition identity: `quire.value.integer-division.euclidean/v1`; revision:
`1-draft.1`. This is a leaf semantic-profile definition selected alongside
`quire.value.complete/v1`; it has no reverse dependency on that root.

For nonzero divisor `b`, quotient is the unique integer for which remainder
`r = a - b*q` satisfies `0 <= r < abs(b)`. Division by zero is undefined. This
same remainder law always owns `mod` under FR-147.

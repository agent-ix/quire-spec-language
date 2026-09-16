# Floor integer-division definition

Definition identity: `quire.value.integer-division.floor/v1`; revision:
`1-draft.1`. This is a leaf semantic-profile definition selected alongside
`quire.value.complete/v1`; it has no reverse dependency on that root.

For nonzero divisor `b`, quotient is the exact rational `a/b` rounded toward
negative infinity and remainder is `a - b*q`. A nonzero remainder has the sign
of `b` and absolute value less than `abs(b)`. Division by zero is undefined.
`mod` does not inherit this definition and remains Euclidean under FR-147.

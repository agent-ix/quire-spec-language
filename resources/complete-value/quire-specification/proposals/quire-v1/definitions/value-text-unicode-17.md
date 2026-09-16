# Unicode 17 text-profile definition

Definition identity: `quire.value.text.unicode-17.0.0/v1`; revision:
`1-draft.1`. This definition selects the Unicode 17.0.0 normalization data and
algorithms for the `nfc`, `nfd`, `nfkc` and `nfkd` profiles in FR-141.

The normative external closure is the following exact set of Unicode-hosted
artifacts. SHA-256 is over the downloaded response bytes without any newline,
encoding or line-ending rewrite.

| Artifact | Authoritative URL | SHA-256 |
| --- | --- | --- |
| UAX #15 revision 57 (Unicode 17.0.0) | `https://www.unicode.org/reports/tr15/tr15-57.html` | `c0c05f91e1c4f9be3d987e27d76cf254b30003b6be41eb94977cc3fe148d4c4e` |
| `UnicodeData.txt` | `https://www.unicode.org/Public/17.0.0/ucd/UnicodeData.txt` | `2e1efc1dcb59c575eedf5ccae60f95229f706ee6d031835247d843c11d96470c` |
| `DerivedNormalizationProps.txt` | `https://www.unicode.org/Public/17.0.0/ucd/DerivedNormalizationProps.txt` | `71fd6a206a2c0cdd41feb6b7f656aa31091db45e9cedc926985d718397f9e488` |
| `CompositionExclusions.txt` | `https://www.unicode.org/Public/17.0.0/ucd/CompositionExclusions.txt` | `2f239196ef3b5b61db5cc476e9bd80f534d15aa1b74e1be1dea5d042a344c85f` |
| `NormalizationTest.txt` | `https://www.unicode.org/Public/17.0.0/ucd/NormalizationTest.txt` | `5019ffd530751a741900c849c0e010332f142a3612234639bd200b82138a87db` |
| Unicode data license | `https://www.unicode.org/license.txt` | `e7a93b009565cfce55919a381437ac4db883e9da2126fa28b91d12732bc53d96` |

UAX #15 supplies the algorithm, the four UCD files supply the normalization
properties, exclusions and conformance vectors, and the license supplies the
redistribution terms. A producer verifies every digest before compiling or
vendoring tables. Implementations may encode the verified tables differently
only when every Unicode scalar sequence produces the same normalization result
and the complete `NormalizationTest.txt` corpus passes. `unicode-scalars` and
`binary-utf8` select FR-141's non-normalizing relations but retain this
definition when a package also uses a normalization profile.

This is a leaf semantic-profile definition selected alongside
`quire.value.complete/v1`; it has no reverse dependency on that root.

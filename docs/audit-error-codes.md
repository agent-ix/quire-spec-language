# Fixture audit error catalog

The private `fixture-audit` boundary implements FR-012. Errors implement Rust
Display/Error; stderr starts with a stable code and includes contextual prose.
The code is the classification contract; messages may become more specific.
Failed or incomplete checks emit no success summary on stdout.

| Code | Exit | Meaning |
| --- | --- | --- |
| usage | 2 | Missing, unknown, extra or malformed mode/arguments. |
| io | 2 | Selected file or output could not be read/written. |
| invalid-json | 1 | Malformed/trailing JSON, invalid Unicode, or duplicate decoded keys. |
| invalid-fixture | 1 | Wrong required field shape or failed selected correspondence/control. |
| digest-mismatch | 1 | Selected bytes do not match their digest. |
| profile-mismatch | 1 | Review fixture/profile version differs from the selected reference. |
| identity-mismatch | 1 | Payload identity/revision differs from its reference or package. |
| identity-content-conflict | 1 | Immutable review key reused with a different profile/digest. |
| foreign-path | 1 | Absolute manifest path or canonical target outside the selected root. |
| resource-exhausted | 3 | Byte, file, decoded-value, JSON-depth or native syntax ceiling exceeded. |
| producer-language-unapproved | 3 | Fresh TypeSpec/Node production has no owner language disposition. |

Zero reports only the mode's completed byte/identity/correspondence/syntax checks.
These codes do not implement the shared portable verification result envelope.
The roles mode's fixed adjacent profile definition follows FR-012's explicit
layout; manifest-controlled locators still stay inside the fixture directory.

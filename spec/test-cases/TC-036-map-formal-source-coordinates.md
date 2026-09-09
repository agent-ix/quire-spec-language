---
id: TC-036
title: "Map independent source coordinate examples"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-014
    type: verifies
---
# TC-036: Map independent source coordinate examples

## Description

Integration, P1; verifies FR-014-AC-2 against independently authored endpoints.

## Test Procedure

Map spans in the exact UTF-8 string a\r\né😀\nz, including both boundaries of
CRLF, each multibyte scalar, the final character and EOF. Include the empty
source and empty spans. Read 1 MiB of ASCII through Source and map its EOF;
attempt one additional source byte and verify the existing intake refusal.

## Expected Results

Every returned IR endpoint retains the selected formal identity. Byte 3 is line
2 column 1; byte 5 is line 2 column 2; byte 9 is line 2 column 3; byte 10 is line
3 column 1; EOF byte 11 is line 3 column 2. Empty source byte 0 is (1, 1).
ASCII byte 1,048,576 is (1, 1,048,577); adding a byte refuses at source intake.

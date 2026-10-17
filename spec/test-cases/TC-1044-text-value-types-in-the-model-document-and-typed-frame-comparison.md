---
id: TC-1044
title: "A Text value type is declared in the model document, bounds its values and is compared by its declared type"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: verifies
---
# TC-1044: A Text value type is declared in the model document, bounds its values and is compared by its declared type

## Description

Verify the model-document form of a Text value type, the order in which
the Text reader refuses a defective declaration, the value domain every
surface decides on the retained sequence, and the frame check's comparison of
each declared type by that type's own equality.

Scope: FR-056-AC-37, FR-056-AC-38, FR-056-AC-39, FR-106-AC-40 and FR-106-AC-49. The bare
`ix://quire/native/Text` `typeRef` rows stay with TC-897 step 3, which
depends on the semantic-IR validator's native vocabulary naming `Text`; this
case never uses that spelling.

## Preconditions

Every row that admits `Title` or reaches the Text reader (steps 1, 3, 4 and
step 2 rows (b) to (k)) depends on the amended semantic-IR validator that
FR-056's Text paragraph states: a `minLength`, `maxLength` or `format`
constraint applied to a value type of `scalar` `string` is applicable. At the
pinned validator those rows refuse `invalid_model_binding`/
`malformed-declaration` at `Title`. Rows (a), (l), (m), (n) and the field row
are decided by the validator or the reader without it and are reachable today.

- TC-897's base package, with package identity prefix `P`. Where a step says
  `Title`, the package declares the value type below; its constraints are
  written in the member order of RFC 8785, `O` is the origin of any other
  declaration of the package, and `K` stands for `<P as a dotted code>`.
- `Title`: `{"constraints":[Cmin,Cmax,Cfmt],"displayName":"Title",
  "extensions":[],"identity":"ix://P/Title","kind":{"module":"P","name":
  "value_type"},"origin":O,"roles":[],"scalar":"string","unknownPolicy":
  "reject"}`, where `Cmin` is `{"appliesTo":"ix://P/Title","diagnosticCode":
  "K.TITLE_MINLENGTH","identity":"ix://P/constraint/Title/minLength",
  "keyword":"minLength","operands":{"value":1},"origin":O}`, `Cmax` is the
  same with `maxLength`, `TITLE_MAXLENGTH`, identity `.../maxLength` and
  `"value":8`, and `Cfmt` is the same with `format`, `TITLE_PROFILE`,
  identity `.../format` and `"operands":{"name":"text-profile:nfc"}`.
  The object type `Note` declares the field `label: Title` and the operation
  `rename(to: Title): Title`.
- For step 7, TC-1038's package and base snapshot, with `child.aliases`
  holding Text objects `a` and `b` in the pre and the post snapshot of every
  invocation, `retitle`'s `modifies` removed so that the frame grants nothing,
  and TC-1036's `label` (String), `token` (UUID) and `subtitle` (Option of
  `Tag`, `Text[0, 4; unicode-scalars]`) fields, and the field `address` of the
  record `Address { street: Title }` (FR-056-AC-21).

## Test Procedure

1. Admit the package. Repeat with the profile of `Cfmt` set to each of
   `unicode-scalars`, `nfd`, `nfkc`, `nfkd` and `binary-utf8`; with the
   constraints in the order `Cfmt`, `Cmax`, `Cmin`; with `minLength` `0` and
   `maxLength` `0`; with `3` and `3`. Declare `Tag` with `target`
   `ix://quire/native/String` and no constraint, and a field `tag: Tag`.
2. Mutate `Title` one row at a time, each from the base: (a) add the member
   `"textProfile":"nfc"`; (b) add a constraint `{"appliesTo":"ix://P/Title",
   "diagnosticCode":"K.TITLE_PATTERN","identity":"ix://P/constraint/Title/
   pattern","keyword":"pattern","operands":{"dialect":"ecma-262","regex":"^a"},"origin":O}`;
   (c) change `Cfmt`'s name to `email:rfc`; (d) add a second `minLength`
   constraint with identity `.../minLength-2`; (e) remove `Cfmt`; (f) remove
   `Cmin`; (g) remove `Cmax`; (h) set the profile to `nfx`; (i) set the
   profile to `NFC`; (j) set `minLength` `5` and `maxLength` `2`; (k) the same
   with the profile `nfx`; (l) set `minLength` `-1`; (m) set `maxLength`
   `1.5`; (n) set `Cfmt`'s name to `text-profile:`. Separately, give the field
   `label` the member `"textProfile":"nfc"` with `typeRef`
   `ix://quire/native/String` and `Title` unchanged.
3. Admit a snapshot with `child.title` `Text[1, 8; nfc]` holding eight pairs
   of U+0065 U+0301 (sixteen supplied scalars) and then nine pairs, and an
   empty text. Replay a function with one parameter typed `Title`, written as
   FR-098-AC-13 writes its `String` functions, from an `Input` assignment and
   from a `Witness` entry `{"type":"text","value":"<text>"}`, each holding
   eight pairs and then nine. Admit a value type `Bin` bound `0` to `4` under
   `binary-utf8` and a field holding two U+00E9, then three. Put a String
   object in a `Title` slot and a Text object in a String slot.
4. With each of these changes between the pre and post snapshot of an
   invocation of `retitle` that grants nothing, change exactly one field and
   admit: `child.label` from U+00E9 to U+0065 U+0301; `child.title` from
   U+00E9 to U+0065 U+0301 (supplied as written in the post snapshot) and
   from `a` to `b`; `child.subtitle` from U+00E9 to U+0065 U+0301 inside a
   present option; `child.subtitle` from absent to `{"present":<Text a>}`
   and from `{"present":<Text a>}` to the same with the object's members
   written in another order; `child.aliases` from `a`, `b` to `b`, `a`, and
   the same elements with only the document's whitespace changed; `child.token`
   to another canonical UUID; `child.address`'s `street` from U+00E9 to U+0065
   U+0301 and from `Main` to `Side`.

## Expected Results

- Step 1 admits every document. `label`, `rename`'s parameter and its result
  have the value type `Text[1, 8; nfc]` for the base, and `Text[1, 8; p]` for
  profile `p`, then `Text[0, 0; nfc]` and `Text[3, 3; nfc]`; the reordered
  constraints give the same declaration. `tag` is `String`, never `Text`.
- Step 2 refuses each row at `Title` except the last, and admits nothing: (a)
  `invalid_model_binding`/`malformed-declaration` from the validator, the
  schema admitting `textProfile` only on the core string scalar; (b)
  `declaration-form` naming `pattern`; (c) `declaration-form` naming `format`;
  (d) `invalid_model_binding`/`malformed-declaration` naming `minLength`; (e)
  `declaration-form` naming `profile`; (f) `malformed-declaration` naming
  `minLength`; (g) `malformed-declaration` naming `maxLength`; (h) and (i)
  `declaration-form` naming `nfx` and `NFC`; (j) `declaration-form` naming
  the bounds `5` and `2`; (k) `declaration-form` naming `nfx`, the profile
  before the bounds; (l), (m) and (n) `malformed-declaration` decided by the
  semantic-IR validator before the Text reader, the bound or name pattern
  being outside its schema. The last row refuses `declaration-form` at
  `label`, naming `textProfile`.
- Step 3 admits eight pairs by every surface, as eight retained scalars. Nine
  pairs and the empty text refuse `invalid_runtime_input`/`invalid-value` for
  the snapshot, naming `title`, the bounds `1` and `8`, the profile and the
  measured length; the replayed argument refuses `invalid_runtime_input`/`WrongValueKind`
  naming the parameter's position before the call, for the `Input` assignment
  and the `Witness` entry. The two U+00E9 admit under
  `Bin` (four bytes) and three refuse (six bytes). The String object in the
  Text slot and the Text object in the String slot refuse `wrong-value-kind`.
- Step 4 decides each row by its declared type: the `label` change (String)
  refuses `frame_violation`/`unauthorized-change` naming `child` and `label`;
  the `title` change from U+00E9 to U+0065 U+0301 under `nfc` admits as no
  change, and `a` to `b` refuses naming `child` and `title`; the `subtitle`
  change under `unicode-scalars` refuses naming `child` and `subtitle`; absent
  to present refuses naming `child` and `subtitle`, and the reordered members admit; the
  swapped `aliases` refuse naming `child` and `aliases`, and the whitespace
  change admits; the `token` change refuses naming `child` and `token`; the
  `street` change from U+00E9 to U+0065 U+0301 admits as no change, and
  `Main` to `Side` refuses naming `child` and `address`.

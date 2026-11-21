---
id: TC-3013
title: "Systems part, port and allocation records are admitted"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-103
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-086
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-710
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-897
    type: verifies
---
# TC-3013: Systems part, port and allocation records are admitted

## Description

Verify that model intake and the assembler admit a domain package's systems
part, port and allocation records as model structure, that intake reads each
with its own fields, and that a unit protocol can bind a role to the admitted
Port. The source-model variant follows Parts and Ports into a Connection
used by a protocol channel. Scope: FR-103-AC-11, FR-086-AC-2,
FR-897-AC-1, FR-897-AC-3, FR-897-AC-5, FR-710-AC-11 and FR-710-AC-12.

## Test Procedure

1. Build a Semantic IR document for package `test/plant` with interface
   `Flow` (field `rate`), object types `Sys` and `Pump`, part `SysPump`
   (owner `Sys`, declared type `Pump`, multiplicity `[1, 1]`), port `PumpOut`
   (owner `SysPump`, interface type `Flow`, direction `out`, multiplicity
   `[1, 1]`) and allocation `PumpAlloc` (source `PumpOut`, target `SysPump`).
   Admit it and read its records.
2. Assemble and check, in linked mode, the unit `model N = "test/plant";`
   whose protocol `P` declares `role r on N::PumpOut`.

Tag both steps `#[trace("TC-3013", "FR-103-AC-11")]`.

3. Compile this model source, retaining its source labels and spans, and
   inspect the selected model's systems records:

   ```fsl stage=parse
   language "ix:native" edition "1-draft";
   domain "test/plant";
   object Sys { }
   object Pump { }
   object Tank { }
   interface Flow { }
   part SysPump: Pump [1, 1] in Sys;
   part SysTank: Tank [1, 1] in Sys;
   port PumpOut: Flow [1, 1] out on SysPump;
   port TankIn: Flow [1, 1] in on SysTank;
   connection Pipe SysPump.PumpOut -> SysTank.TankIn;
   ```

4. Compile this consuming source in a `native-compile/1` request with a
   `models` entry selecting step 3's source as `native-model-unit/1`.
   Compile it again with that model's compiled document selected as
   `semantic-ir/2.0.0`, using equal source labels for the consuming source:

   ```fsl stage=parse
   language "ix:native" edition "1-draft";
   profile p = "filament.protocol.finite-global/v1";
   model N = "test/plant";
   protocol Pipeline using p over (input: N::Sys) on origin {
     role Outlet on N::PumpOut;
     role Inlet on N::TankIn;
     relationship Link = N::Pipe;
     channel Feed from Outlet to Inlet via Link carries N::Flow
       ordering unordered delivery exactly-once-premise capacity 2 overflow reject;
     run sequence Main { }
     finish Closed as (closed: Boolean) { true };
   }
   ```

5. Repeat step 3 with `<->` and both ports `inout`; repeat step 4 with
   the original model and the channel roles reversed, and with each role's
   member independently changed to `N::SysPump` and to `N::Pipe`.
6. Independently mutate each end in step 3 to the absent qualifier, absent
   Port, wrong qualifier kind, wrong Port kind and mismatched owner forms
   in FR-710-AC-12. Compile each model source without a consuming unit.
7. Independently change step 3's `PumpOut` direction to `in`, add
   `interface Other { marker: Boolean; }` and change `TankIn`'s interface
   to `Other`, and change `PumpOut`'s multiplicity to `[2, 2]`. Compile step
   4 with each model, preserving the selected model's source identity.

Tag steps 3 and 5's bidirectional model `FR-710-AC-11`, step 4
`FR-897-AC-1`, step 5's consuming variants `FR-897-AC-3` and
`FR-897-AC-5` as applicable, step 6 `FR-710-AC-12`, and step 7
`FR-086-AC-2`, each also with `TC-3013`.

## Expected Results

- Step 1: the records hold one Part `SysPump` with owner `Sys`, type `Pump`
  and multiplicity `[1, 1]`, one Port `PumpOut` with owning part `SysPump`,
  interface type `Flow`, direction `out` and multiplicity `[1, 1]`, and one
  Allocation `PumpAlloc` with source `PumpOut` and target `SysPump`.
- Step 2: assembly succeeds with no `UnsupportedModelMember` refusal and
  checking succeeds; protocol `P` has one role whose model member is the
  declaration key `ix://test/plant/PumpOut` and whose model selection is
  `test/plant`.
- Step 3: the selected model retains the Parts, Ports, ownership, types,
  directions and multiplicities in FR-710-AC-11. `Pipe` names
  `ix://test/plant/PumpOut` and `ix://test/plant/TankIn` as its ends,
  in that order; neither end identity includes its owning Part.
- Step 4: both requests succeed and emit equal admitted checked-package
  bytes. The two role members retain the distinct Port keys under selection
  `test/plant`; `Link` retains `ix://test/plant/Pipe` and its ordered ends;
  the channel's `connection` references that relationship node, which is
  listed in its dependencies.
- Step 5: the bidirectional model admits with bidirectional direction and
  unchanged Port end keys. Reversed channel roles and non-Port role
  members refuse `invalid_model_binding`/`malformed-declaration` at
  `Link` or the role's member reference respectively, without a checked
  package, as FR-897 requires.
- Step 6: each mutation refuses at its offending name in the model source,
  with FR-710-AC-12's code, cause and retained keys, without a document.
- Step 7: binding `N::Pipe` refuses with the independent direction,
  interface-conformance or multiplicity cause of FR-086:
  `invalid_model_binding`/`port-direction`, `ill_typed`/`type-mismatch`
  or `ill_typed`/`multiplicity-narrowing`, respectively, retaining the
  Connection key and offending condition, without a checked package.

These cases use original declarations only. Specialization, effective
declaration identity and inheritance remain under their existing owning
requirements; this case adds no such semantics. The original allocation
checks in steps 1 and 2 remain required.

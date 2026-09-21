---
id: TC-217
title: "The model binder is a pure function with no cross-run ambient state"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-081
    type: verifies
---
# TC-217: The model binder is a pure function with no cross-run ambient state

## Description

Verify that the model binder's output depends only on its explicit
`DomainPackage` and limits arguments, with no observable dependency on prior
calls or on any static/global table. Scope: FR-081-AC-5.

Catches an implementation that caches a declaration or an intermediate
derivation fact in a process-global or static table keyed by something
weaker than the full `DomainPackage` value (for example, keyed only by
domain-package identity and version, ignoring the digest) — a defect that a
single-call test can never observe, because the first call always populates
the cache correctly; it only surfaces when two different packages sharing an
identity/version but differing digest are bound in the same process and the
second run's result is compared against a fresh, isolated run of the same
input.

## Test Procedure

1. In one process, run the model binder over domain package P (a fixed
   `DomainPackage` value and fixed limits) and record its full output
   (original keys, effective identities, ordering).
2. In the same process, immediately run the model binder over a second,
   unrelated domain package Q that shares P's identity and version label but
   has a different digest and different declarations, and record its output.
3. In the same process, run the model binder over P again and record its
   output a second time.
4. In an independent process invocation (a fresh process, no shared state
   with steps 1–3), run the model binder over P alone and record its output.
5. Compare: the step-1 output to the step-3 output (same-process
   repetition), and the step-1 output to the step-4 output
   (cross-process).

## Expected Results

Step 3's output is byte-identical to step 1's, unaffected by step 2's
intervening call over Q. Step 4's output is byte-identical to step 1's. A
mutant that caches by a weaker key than the full `DomainPackage` produces a
step-3 output containing traces of Q's declarations (or fails to reflect P
correctly the second time), failing the first comparison.

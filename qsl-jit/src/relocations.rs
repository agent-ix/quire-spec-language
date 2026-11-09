// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-561-AC-3: the call targets of every generated block, read from the code
//! generator's relocation records.
//!
//! A block reaches the kernel only through the module's imported kernel
//! shim and reaches its own blocks by direct calls, and the translator emits
//! no indirect call. [`BlockRelocations`] holds the direct targets and the
//! count of indirect calls of one block, and [`check_call_targets`] fails on
//! any target that is neither the shim nor a block, and on any indirect call.
//!
//! The records are read only by the `test-support` check, so the whole
//! module is built only with that feature.

use cranelift_codegen::ir::{ExternalName, Function, InstructionData, UserExternalName};
use cranelift_codegen::CompiledCode;
use cranelift_module::{FuncId, Module};

use crate::exec::KERNEL_SHIM_SYMBOL;

/// What a block's direct call target is.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CallTarget {
    /// The named entry of the `KernelTable` of the same prepared value.
    KernelTable(String),
    /// The named block of the same prepared value.
    Block(String),
    /// A target in neither (named as the code generator names it).
    Outside(String),
}

/// The call-target facts of one generated block.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlockRelocations {
    /// The block's symbol.
    pub block: String,
    /// Its direct call targets, from its relocation records.
    pub direct: Vec<CallTarget>,
    /// How many indirect calls it makes. The translator emits none.
    pub indirect_calls: usize,
}

/// A call target that is neither a `KernelTable` entry nor a block of the
/// prepared value, or an indirect call.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StrayCall {
    /// The block that makes the call.
    pub block: String,
    /// The target, as the records name it.
    pub target: String,
}

/// What a test code generator emits besides the translation (FR-561-AC-3).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StrayCallKind {
    /// One direct call to an address outside the kernel table and the
    /// prepared blocks.
    Direct,
    /// One indirect call through a register.
    Indirect,
}

/// A block as the translator defined it, before its targets are classified
/// against the set of blocks the prepared value holds.
pub(crate) struct RawBlock {
    block: String,
    direct: Vec<String>,
    indirect_calls: usize,
}

impl RawBlock {
    /// The records of the function `id` just defined from `function` and
    /// compiled to `code`.
    pub(crate) fn of(
        module: &impl Module,
        id: FuncId,
        function: &Function,
        code: &CompiledCode,
    ) -> Self {
        let name_of = |name: &ExternalName| -> String {
            match name {
                ExternalName::User(reference) => {
                    let UserExternalName { namespace, index } =
                        function.params.user_named_funcs()[*reference].clone();
                    let target = (namespace == 0).then(|| FuncId::from_u32(index));
                    target
                        .and_then(|target| {
                            module.declarations().get_function_decl(target).name.clone()
                        })
                        .unwrap_or_else(|| format!("user:{namespace}:{index}"))
                }
                other => format!("{other:?}"),
            }
        };
        let direct = code
            .buffer
            .relocs()
            .iter()
            .filter_map(|record| match &record.target {
                cranelift_codegen::RelocTarget::ExternalName(name) => Some(name_of(name)),
                cranelift_codegen::RelocTarget::Label(_) => None,
            })
            .collect();
        Self {
            block: module
                .declarations()
                .get_function_decl(id)
                .name
                .clone()
                .unwrap_or_default(),
            direct,
            indirect_calls: indirect_calls(function),
        }
    }

    /// The records with each direct target classified against `blocks`, the
    /// symbols of the blocks the prepared value holds.
    pub(crate) fn classify(self, blocks: &[String]) -> BlockRelocations {
        BlockRelocations {
            block: self.block,
            direct: self
                .direct
                .into_iter()
                .map(|name| {
                    if name == KERNEL_SHIM_SYMBOL {
                        CallTarget::KernelTable(name)
                    } else if blocks.contains(&name) {
                        CallTarget::Block(name)
                    } else {
                        CallTarget::Outside(name)
                    }
                })
                .collect(),
            indirect_calls: self.indirect_calls,
        }
    }

    /// The block's symbol.
    pub(crate) fn name(&self) -> &str {
        &self.block
    }
}

/// How many indirect calls `function` makes.
fn indirect_calls(function: &Function) -> usize {
    let dfg = &function.dfg;
    function
        .layout
        .blocks()
        .flat_map(|block| function.layout.block_insts(block))
        .filter(|inst| matches!(dfg.insts[*inst], InstructionData::CallIndirect { .. }))
        .count()
}

/// Check that every direct target of `blocks` is a `KernelTable` entry or a
/// block of the same prepared value and no block calls indirectly, and count the blocks that call a `KernelTable` entry.
///
/// # Errors
/// The first stray call, naming its block and target.
pub fn check_call_targets(blocks: &[BlockRelocations]) -> Result<usize, StrayCall> {
    let mut calling_the_table = 0;
    for block in blocks {
        if let Some(stray) = block.direct.iter().find_map(|target| match target {
            CallTarget::Outside(target) => Some(target.clone()),
            CallTarget::KernelTable(_) | CallTarget::Block(_) => None,
        }) {
            return Err(StrayCall {
                block: block.block.clone(),
                target: stray,
            });
        }
        if block.indirect_calls > 0 {
            return Err(StrayCall {
                block: block.block.clone(),
                target: "indirect call: the translator emits none".to_owned(),
            });
        }
        if block
            .direct
            .iter()
            .any(|target| matches!(target, CallTarget::KernelTable(_)))
        {
            calling_the_table += 1;
        }
    }
    Ok(calling_the_table)
}

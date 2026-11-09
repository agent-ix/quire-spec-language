// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-561-AC-3: the call targets of every generated block, read from the code
//! generator's relocation records.
//!
//! A block reaches the kernel only through the module's imported kernel
//! shim, which Cranelift names with an absolute relocation, and reaches its
//! own blocks with relative call relocations. [`BlockRelocations`] holds those
//! records for one block and [`check_call_targets`] fails on any target that
//! is neither.

use cranelift_codegen::binemit::Reloc;
use cranelift_codegen::ir::{
    self, ExternalName, Function, InstructionData, Opcode, UserExternalName, ValueDef,
};
use cranelift_codegen::CompiledCode;
use cranelift_module::{FuncId, Module};

/// The symbol of the module's one kernel table entry (FR-561).
pub(crate) const KERNEL_SHIM_SYMBOL: &str = "qsl_jit_shim";

/// How a relocation names its target.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RelocationKind {
    /// The target's absolute address, as a call through an imported symbol
    /// loads it.
    Absolute,
    /// A pc-relative call to a function placed in the same module.
    Relative,
}

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

/// Where an indirect call's target register was loaded from.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IndirectSource {
    /// A `KernelTable` slot of the same prepared value.
    TableSlot,
    /// Any other register.
    Register,
}

/// One relocation of a block that names a call target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Relocation {
    /// How the record names its target.
    pub kind: RelocationKind,
    /// The target.
    pub target: CallTarget,
}

/// The call-target facts of one generated block.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlockRelocations {
    /// The block's symbol.
    pub block: String,
    /// Its direct call targets, from its relocation records.
    pub direct: Vec<Relocation>,
    /// The source of each of its indirect calls' targets.
    pub indirect: Vec<IndirectSource>,
}

#[cfg(feature = "test-support")]
/// A call target that is neither a `KernelTable` entry nor a block of the
/// prepared value, or an indirect call target not loaded from a table slot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StrayCall {
    /// The block that makes the call.
    pub block: String,
    /// The target, as the records name it.
    pub target: String,
}

/// What a test code generator emits besides the translation (FR-561-AC-3).
#[cfg(feature = "test-support")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StrayCallKind {
    /// One direct call to an address outside the kernel table and the
    /// prepared blocks.
    Direct,
    /// One indirect call through a register not loaded from a table slot.
    Indirect,
}

/// A block as the translator defined it, before its targets are classified
/// against the set of blocks the prepared value holds.
pub(crate) struct RawBlock {
    block: String,
    direct: Vec<(RelocationKind, String)>,
    indirect: Vec<IndirectSource>,
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
            .filter_map(|record| {
                let kind = match record.kind {
                    Reloc::Abs4 | Reloc::Abs8 => RelocationKind::Absolute,
                    _ => RelocationKind::Relative,
                };
                match &record.target {
                    cranelift_codegen::RelocTarget::ExternalName(name) => {
                        Some((kind, name_of(name)))
                    }
                    cranelift_codegen::RelocTarget::Label(_) => None,
                }
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
            indirect: indirect_sources(function),
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
                .map(|(kind, name)| Relocation {
                    kind,
                    target: if name == KERNEL_SHIM_SYMBOL {
                        CallTarget::KernelTable(name)
                    } else if blocks.contains(&name) {
                        CallTarget::Block(name)
                    } else {
                        CallTarget::Outside(name)
                    },
                })
                .collect(),
            indirect: self.indirect,
        }
    }

    /// The block's symbol.
    pub(crate) fn name(&self) -> &str {
        &self.block
    }
}

/// The source of each indirect call target of `function`: a load through a
/// symbol value is a table slot, anything else is a register.
fn indirect_sources(function: &Function) -> Vec<IndirectSource> {
    let dfg = &function.dfg;
    function
        .layout
        .blocks()
        .flat_map(|block| function.layout.block_insts(block))
        .filter_map(|inst| match &dfg.insts[inst] {
            InstructionData::CallIndirect { args, .. } => {
                let target = args.as_slice(&dfg.value_lists).first().copied()?;
                Some(match dfg.value_def(target) {
                    ValueDef::Result(loaded, _) if is_table_slot_load(function, loaded) => {
                        IndirectSource::TableSlot
                    }
                    _ => IndirectSource::Register,
                })
            }
            _ => None,
        })
        .collect()
}

/// Whether `inst` loads from the address of a symbol (a table slot).
fn is_table_slot_load(function: &Function, inst: ir::Inst) -> bool {
    let dfg = &function.dfg;
    let InstructionData::Load {
        opcode: Opcode::Load,
        arg,
        ..
    } = &dfg.insts[inst]
    else {
        return false;
    };
    matches!(
        dfg.value_def(*arg),
        ValueDef::Result(base, _)
            if matches!(dfg.insts[base], InstructionData::UnaryGlobalValue { .. })
    )
}

#[cfg(feature = "test-support")]
/// Check that every direct target of `blocks` is a `KernelTable` entry or a
/// block of the same prepared value and every indirect target is loaded from
/// a table slot, and count the blocks that call a `KernelTable` entry.
///
/// # Errors
/// The first stray call, naming its block and target.
pub fn check_call_targets(blocks: &[BlockRelocations]) -> Result<usize, StrayCall> {
    let mut calling_the_table = 0;
    for block in blocks {
        if let Some(stray) = block.direct.iter().find_map(|record| match &record.target {
            CallTarget::Outside(target) => Some(target.clone()),
            CallTarget::KernelTable(_) | CallTarget::Block(_) => None,
        }) {
            return Err(StrayCall {
                block: block.block.clone(),
                target: stray,
            });
        }
        if block.indirect.contains(&IndirectSource::Register) {
            return Err(StrayCall {
                block: block.block.clone(),
                target: "indirect call through a register not loaded from a table slot".to_owned(),
            });
        }
        if block
            .direct
            .iter()
            .any(|record| matches!(record.target, CallTarget::KernelTable(_)))
        {
            calling_the_table += 1;
        }
    }
    Ok(calling_the_table)
}

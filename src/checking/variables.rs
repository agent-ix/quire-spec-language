// SPDX-License-Identifier: AGPL-3.0-only
//! FR-016/040: contextual type variables; callers own syntax and work limits.

use super::NativeType;

/// Two established nominal types cannot belong to the same constraint class.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct TypeConflict;

/// Union-find variables local to one checker invocation.
pub(super) struct Variables<'a> {
    parents: Vec<usize>,
    ranks: Vec<u8>,
    known: Vec<Option<NativeType<'a>>>,
}

impl<'a> Variables<'a> {
    pub(super) fn new(count: usize) -> Self {
        Self {
            parents: (0..count).collect(),
            ranks: vec![0; count],
            known: vec![None; count],
        }
    }
    pub(super) fn len(&self) -> usize {
        self.parents.len()
    }
    pub(super) fn fresh(&mut self) -> usize {
        let id = self.parents.len();
        self.parents.push(id);
        self.ranks.push(0);
        self.known.push(None);
        id
    }
    pub(super) fn root(&mut self, mut var: usize) -> usize {
        while self.parents[var] != var {
            self.parents[var] = self.parents[self.parents[var]];
            var = self.parents[var];
        }
        var
    }
    pub(super) fn get(&mut self, var: usize) -> Option<NativeType<'a>> {
        let root = self.root(var);
        self.known[root].clone()
    }
    /// Borrow before a caller charges the wrapper/string work of copying a type.
    pub(super) fn peek(&mut self, var: usize) -> Option<&NativeType<'a>> {
        let root = self.root(var);
        self.known[root].as_ref()
    }
    pub(super) fn assign(
        &mut self,
        var: usize,
        ty: NativeType<'a>,
    ) -> std::result::Result<bool, TypeConflict> {
        let root = self.root(var);
        match &self.known[root] {
            Some(prior) if prior != &ty => Err(TypeConflict),
            Some(_) => Ok(false),
            None => {
                self.known[root] = Some(ty);
                Ok(true)
            }
        }
    }
    pub(super) fn unify(
        &mut self,
        left: usize,
        right: usize,
    ) -> std::result::Result<(), TypeConflict> {
        let mut a = self.root(left);
        let mut b = self.root(right);
        if a == b {
            return Ok(());
        }
        if matches!((&self.known[a], &self.known[b]), (Some(a), Some(b)) if a != b) {
            return Err(TypeConflict);
        }
        if self.ranks[a] < self.ranks[b] {
            std::mem::swap(&mut a, &mut b);
        }
        self.parents[b] = a;
        if self.ranks[a] == self.ranks[b] {
            self.ranks[a] += 1;
        }
        if self.known[a].is_none() {
            self.known[a] = self.known[b].take();
        }
        Ok(())
    }
}

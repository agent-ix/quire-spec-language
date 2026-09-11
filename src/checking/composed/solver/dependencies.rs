// SPDX-License-Identifier: AGPL-3.0-only
//! Complete forward signatures through one reverse-edge traversal.

use super::*;
use std::collections::VecDeque;

struct Dependents {
    incoming: Vec<(usize, Site)>,
    remaining: usize,
}

pub(super) fn finish(report: &mut TypeReport<'_, '_>, work: &mut Work, site: Site) -> Result<()> {
    let mut states = Vec::new();
    for output in &report.declarations {
        work.charge(D::Constraints, 1, site)?;
        work.charge(D::Records, 1, site)?;
        let entry = report
            .binding
            .namespace()
            .declaration(output.declaration)
            .expect("bound owner");
        states.push(Dependents {
            incoming: Vec::new(),
            remaining: entry.references().len(),
        });
    }
    for (index, output) in report.declarations.iter().enumerate() {
        let entry = report
            .binding
            .namespace()
            .declaration(output.declaration)
            .expect("bound owner");
        for reference in entry.references() {
            let site = Site {
                declaration: output.declaration,
                unit: entry.unit(),
                expression: match reference.site {
                    DependencySite::Expression(id) => Some(id),
                    DependencySite::Control(_) | DependencySite::Declaration => None,
                },
                span: reference.name.span,
            };
            work.charge(D::Edges, 1, site)?;
            if let Some(target) = reference.target {
                // Upstream binding can exhaust while creating declaration records.
                // Its namespace still names later declarations; an unvisited target
                // keeps this caller's edge unsatisfied rather than indexing past
                // the retained type evidence or inventing a successful target.
                let Some(target) = states.get_mut(target.index()) else {
                    continue;
                };
                // Every occurrence remains an edge; repeated calls retain their
                // own cause locus without copying a target's complete evidence.
                work.charge(D::Records, 1, site)?;
                target.incoming.push((index, site));
            }
        }
    }
    let mut queue = VecDeque::new();
    for (index, output) in report.declarations.iter().enumerate() {
        work.charge(D::Constraints, 1, site)?;
        if output.disposition() != TypeDisposition::Unfinished {
            work.charge(D::Records, 1, site)?;
            queue.push_back(index);
        }
    }
    while let Some(target) = queue.pop_front() {
        let disposition = report.declarations[target].disposition();
        for edge in 0..states[target].incoming.len() {
            let (caller, site) = states[target].incoming[edge];
            work.charge(D::Edges, 1, site)?;
            let was_unfinished =
                report.declarations[caller].disposition() == TypeDisposition::Unfinished;
            match disposition {
                TypeDisposition::Refused => {
                    work.charge(D::Records, 1, site)?;
                    let target = report.declarations[target].declaration;
                    report.declarations[caller].causes.push(TypeCause {
                        site,
                        kind: CauseKind::Dependency { target },
                        profile: 0,
                    });
                }
                TypeDisposition::Typed => {
                    states[caller].remaining -= 1;
                    if states[caller].remaining == 0 && report.declarations[caller].local_complete {
                        report.declarations[caller].complete = true;
                    }
                }
                TypeDisposition::Unfinished => {
                    unreachable!("only settled type dispositions enter the queue")
                }
            }
            if was_unfinished
                && report.declarations[caller].disposition() != TypeDisposition::Unfinished
            {
                work.charge(D::Records, 1, site)?;
                queue.push_back(caller);
            }
        }
    }
    Ok(())
}

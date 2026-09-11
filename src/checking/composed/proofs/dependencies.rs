// SPDX-License-Identifier: AGPL-3.0-only
//! Bounded native totality propagation; unvisited targets remain unfinished.

use super::work::{Dimension as D, Work};
use super::*;
use crate::linking::composed::DependencySite;
use std::collections::VecDeque;

struct State {
    incoming: Vec<(usize, Site)>,
    remaining: usize,
}

pub(super) fn finish(
    report: &mut ProofReport<'_, '_, '_>,
    work: &mut Work,
    site: Site,
) -> Result<(), Exhaustion> {
    let namespace = report.types.binding().namespace();
    let mut states = Vec::new();
    for output in &report.declarations {
        work.charge(D::Types, 1, site)?;
        work.charge(D::Records, 1, site)?;
        states.push(State {
            incoming: Vec::new(),
            remaining: namespace
                .declaration(output.declaration)
                .expect("owner")
                .references()
                .len(),
        });
    }
    for (index, output) in report.declarations.iter().enumerate() {
        for reference in namespace
            .declaration(output.declaration)
            .expect("owner")
            .references()
        {
            let site = Site {
                declaration: output.declaration,
                unit: output.unit,
                expression: match reference.site {
                    DependencySite::Expression(id) => Some(id),
                    DependencySite::Declaration | DependencySite::Control(_) => None,
                },
                span: reference.name.span,
            };
            work.charge(D::Edges, 1, site)?;
            if let Some(target) = reference.target {
                if let Some(state) = states.get_mut(target.index()) {
                    work.charge(D::Records, 1, site)?;
                    state.incoming.push((index, site));
                }
            }
        }
    }
    let mut queue = VecDeque::new();
    for (index, output) in report.declarations.iter().enumerate() {
        work.charge(D::Types, 1, site)?;
        if output.disposition() != ProofDisposition::Unfinished {
            work.charge(D::Records, 1, site)?;
            queue.push_back(index);
        }
    }
    while let Some(target) = queue.pop_front() {
        let disposition = report.declarations[target].disposition();
        for edge in 0..states[target].incoming.len() {
            let (caller, site) = states[target].incoming[edge];
            work.charge(D::Edges, 1, site)?;
            let unfinished =
                report.declarations[caller].disposition() == ProofDisposition::Unfinished;
            match disposition {
                ProofDisposition::Refused => {
                    let target = report.declarations[target].declaration;
                    work.charge(D::Records, 1, site)?;
                    report.declarations[caller].causes.push(Cause {
                        site,
                        kind: CauseKind::Dependency { target },
                    });
                }
                ProofDisposition::Discharged => {
                    states[caller].remaining -= 1;
                    if states[caller].remaining == 0
                        && report.declarations[caller].local_complete
                        && report.declarations[caller].causes.is_empty()
                    {
                        report.declarations[caller].complete = true;
                    }
                }
                ProofDisposition::Unfinished => {
                    unreachable!("only settled dispositions enter the queue")
                }
            }
            if unfinished
                && report.declarations[caller].disposition() != ProofDisposition::Unfinished
            {
                work.charge(D::Records, 1, site)?;
                queue.push_back(caller);
            }
        }
    }
    Ok(())
}

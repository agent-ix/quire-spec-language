// SPDX-License-Identifier: AGPL-3.0-or-later
//! The process's peak resident set size, for the probe's one-shot runs.

/// This process's peak resident set size in KiB (`VmHWM` in
/// `/proc/self/status`), or `None` where that file or line is absent (any
/// non-Linux host). The figure is process-wide and monotone: a probe run
/// that measures several inputs in one process reports the peak of all of
/// them, so the probe measures one input per process.
///
/// `#[string_edge]`: reads the kernel's text status file by its field
/// label; it selects no family semantics.
#[qsl_attrs::string_edge]
pub fn peak_rss_kib() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    status.lines().find_map(|line| {
        line.strip_prefix("VmHWM:")?
            .trim()
            .strip_suffix("kB")?
            .trim()
            .parse()
            .ok()
    })
}

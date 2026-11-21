//! CPU-thread frame accounting. Counts describe encoded commands, even on discard.
//! Upload bytes cover typed Velum buffer writes and explicit external declarations.
//! Glyphon reports text draw invocations; its internal draws/uploads are opaque.
use std::{cell::RefCell, time::Duration};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FrameOutcome {
    #[default]
    Recording,
    Submitted,
    Abandoned,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub context: String,
    pub message: String,
}
#[derive(Clone, Debug, Default)]
pub struct FrameReport {
    pub outcome: FrameOutcome,
    pub render_passes: u64,
    pub compute_passes: u64,
    pub draw_calls: u64,
    pub dispatches: u64,
    pub batches: u64,
    pub instances: u64,
    pub uploaded_bytes: u64,
    pub instance_allocations: u64,
    /// Logical buffer allocations and reserved arena pages for the queried context.
    pub buffers: Option<super::buffer::BufferMemoryStats>,
    /// Retained legacy instance-cache capacity; excludes arena page slack and physical GPU memory.
    pub retained_instance_slots: usize,
    pub retained_instance_bytes: u64,
    /// Measured draw preparation and CPU command encoding, not GPU execution.
    pub preparation_cpu: Duration,
    pub encoding_cpu: Duration,
    /// Complete operation duration, including preparation inside that operation.
    pub pass_cpu: Duration,
    /// Successful Glyphon render invocations; internal command counts are unavailable.
    pub text_draws: u64,
    pub diagnostics: Vec<Diagnostic>,
}
thread_local! { static FRAME: RefCell<FrameReport> = RefCell::default(); }
pub(crate) fn reset() {
    FRAME.with_borrow_mut(|r| *r = FrameReport::default());
}
/// Begin accounting before preparing resources for an image.
pub fn begin() {
    reset();
}
pub fn snapshot() -> FrameReport {
    snapshot_in(None)
}
pub fn snapshot_in(context: Option<super::context::ContextId>) -> FrameReport {
    let mut result = FRAME.with_borrow(Clone::clone);
    (
        result.retained_instance_slots,
        result.retained_instance_bytes,
    ) = super::instances::memory_stats();
    result.buffers = super::buffer::memory_stats(context);
    result
}
pub fn submitted() {
    FRAME.with_borrow_mut(|r| r.outcome = FrameOutcome::Submitted);
}
pub fn abandoned() {
    FRAME.with_borrow_mut(|r| r.outcome = FrameOutcome::Abandoned);
}
pub fn diagnostic(context: impl Into<String>, message: impl Into<String>) {
    FRAME.with_borrow_mut(|r| {
        r.diagnostics.push(Diagnostic {
            context: context.into(),
            message: message.into(),
        })
    });
}
pub fn record_instances(instances: u64) {
    FRAME.with_borrow_mut(|r| r.instances += instances);
}
pub(crate) fn record_draw() {
    FRAME.with_borrow_mut(|r| r.draw_calls += 1);
}
pub(crate) fn record_batch() {
    FRAME.with_borrow_mut(|r| r.batches += 1);
}
pub(crate) fn record_allocation() {
    FRAME.with_borrow_mut(|r| r.instance_allocations += 1);
}
pub(crate) fn record_upload(bytes: u64) {
    FRAME.with_borrow_mut(|r| r.uploaded_bytes += bytes);
}
pub fn record_dispatch() {
    FRAME.with_borrow_mut(|r| r.dispatches += 1);
}
pub fn record_preparation(cpu: Duration) {
    FRAME.with_borrow_mut(|r| r.preparation_cpu += cpu);
}
pub fn record_encoding(cpu: Duration) {
    FRAME.with_borrow_mut(|r| r.encoding_cpu += cpu);
}
pub(crate) fn record_pass(cpu: Duration) {
    FRAME.with_borrow_mut(|r| r.pass_cpu += cpu);
}
pub(crate) fn render_pass() {
    FRAME.with_borrow_mut(|r| r.render_passes += 1);
}
pub(crate) fn compute_pass() {
    FRAME.with_borrow_mut(|r| r.compute_passes += 1);
}
pub(crate) fn text_draw() {
    FRAME.with_borrow_mut(|r| r.text_draws += 1);
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ExternalCounters {
    pub draw_calls: u64,
    pub instances: u64,
    pub uploaded_bytes: u64,
    pub render_passes: u64,
    pub compute_passes: u64,
    pub batches: u64,
}
/// Custom operations can report commands not encoded through Velum helpers.
pub fn record_external(counters: ExternalCounters) {
    FRAME.with_borrow_mut(|r| {
        r.draw_calls += counters.draw_calls;
        r.instances += counters.instances;
        r.uploaded_bytes += counters.uploaded_bytes;
        r.render_passes += counters.render_passes;
        r.compute_passes += counters.compute_passes;
        r.batches += counters.batches;
    });
}

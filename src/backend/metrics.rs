use std::cell::Cell;

/// Counts geometry and fullscreen effects; excludes Glyphon and egui internals.
#[derive(Clone, Copy, Default)]
pub struct RenderMetrics {
    pub batches: u64,
    pub draw_calls: u64,
    pub instance_allocations: u64,
    pub uploaded_bytes: u64,
}

thread_local! {
    static FRAME: Cell<RenderMetrics> = Cell::default();
}

pub fn reset() {
    FRAME.set(RenderMetrics::default());
}

pub fn snapshot() -> RenderMetrics {
    FRAME.get()
}

pub(super) fn batch() {
    FRAME.update(|mut m| { m.batches += 1; m });
}

pub(crate) fn draw() {
    FRAME.update(|mut m| { m.draw_calls += 1; m });
}

pub(super) fn allocation() {
    FRAME.update(|mut m| { m.instance_allocations += 1; m });
}

pub(super) fn upload(bytes: u64) {
    FRAME.update(|mut m| { m.uploaded_bytes += bytes; m });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_counts_distinguish_batches_chunks_and_allocations_then_reset() {
        reset();
        batch();
        for _ in 0..7 {
            draw();
            allocation();
        }
        upload(100_000 * 64);
        let m = snapshot();
        assert_eq!(m.batches, 1);
        assert_eq!(m.draw_calls, 7);
        assert_eq!(m.instance_allocations, 7);
        assert_eq!(m.uploaded_bytes, 6_400_000);
        reset();
        let m = snapshot();
        assert_eq!(m.batches + m.draw_calls + m.instance_allocations + m.uploaded_bytes, 0);
    }
}

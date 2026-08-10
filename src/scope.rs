//! Real-time-safe channel for streaming (pre-clip, post-clip) sample pairs from the audio thread
//! to the GUI thread, for the oscilloscope widget in `src/editor/scope.rs`.
//!
//! Backed by `rtrb`, a wait-free single-producer/single-consumer ring buffer: pushing on the
//! audio thread never allocates or blocks, matching the same real-time constraint the rest of
//! `process()` already follows (see `CLAUDE.md`, priority 2).

use std::sync::{Arc, Mutex};

/// One frame of scope data: the sample right before clipping and right after, for one channel.
#[derive(Clone, Copy, Debug, Default)]
pub struct ScopeFrame {
    pub input: f32,
    pub output: f32,
}

/// Audio-thread producer half. Owned directly by `Openclip` (not shared), so pushing needs no lock.
pub struct ScopeProducer {
    inner: rtrb::Producer<ScopeFrame>,
}

impl ScopeProducer {
    /// Pushes a frame. If the GUI hasn't drained fast enough and the buffer is full, the frame is
    /// silently dropped rather than blocking — the audio thread must never wait on the GUI.
    #[inline]
    pub fn push(&mut self, input: f32, output: f32) {
        let _ = self.inner.push(ScopeFrame { input, output });
    }
}

/// GUI-thread consumer half, shared via `Arc<Mutex<_>>` so it can be cloned into the editor
/// closure the same way `editor_state` already is. The mutex is only ever touched off the audio
/// thread, so locking here doesn't violate `process()`'s real-time constraints.
#[derive(Clone)]
pub struct ScopeConsumer {
    inner: Arc<Mutex<rtrb::Consumer<ScopeFrame>>>,
}

impl ScopeConsumer {
    /// Drains everything currently available and appends it to `dest`, in order.
    pub fn drain_into(&self, dest: &mut Vec<ScopeFrame>) {
        let mut consumer = self.inner.lock().unwrap();
        while let Ok(frame) = consumer.pop() {
            dest.push(frame);
        }
    }
}

/// Creates a linked producer/consumer pair with room for `capacity` frames.
pub fn channel(capacity: usize) -> (ScopeProducer, ScopeConsumer) {
    let (producer, consumer) = rtrb::RingBuffer::new(capacity);
    (
        ScopeProducer { inner: producer },
        ScopeConsumer {
            inner: Arc::new(Mutex::new(consumer)),
        },
    )
}

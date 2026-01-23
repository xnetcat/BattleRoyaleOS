//! Synchronization primitives for SMP

use core::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

/// A simple spinlock
pub struct SpinLock {
    locked: AtomicBool,
}

impl SpinLock {
    pub const fn new() -> Self {
        Self {
            locked: AtomicBool::new(false),
        }
    }

    /// Acquire the lock
    pub fn lock(&self) {
        while self
            .locked
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            // Spin with a hint
            while self.locked.load(Ordering::Relaxed) {
                core::hint::spin_loop();
            }
        }
    }

    /// Try to acquire the lock (non-blocking)
    pub fn try_lock(&self) -> bool {
        self.locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
    }

    /// Release the lock
    pub fn unlock(&self) {
        self.locked.store(false, Ordering::Release);
    }
}

/// A barrier for synchronizing multiple cores
pub struct CoreBarrier {
    count: AtomicUsize,
    target: usize,
    generation: AtomicU32,
}

impl CoreBarrier {
    pub const fn new(num_cores: usize) -> Self {
        Self {
            count: AtomicUsize::new(0),
            target: num_cores,
            generation: AtomicU32::new(0),
        }
    }

    /// Wait at the barrier until all cores have arrived
    pub fn wait(&self) {
        let current_gen = self.generation.load(Ordering::Acquire);

        // Increment the count
        let arrived = self.count.fetch_add(1, Ordering::AcqRel) + 1;

        if arrived == self.target {
            // Last core to arrive - reset and advance generation
            self.count.store(0, Ordering::Release);
            self.generation.fetch_add(1, Ordering::Release);
        } else {
            // Wait for generation to change
            while self.generation.load(Ordering::Acquire) == current_gen {
                core::hint::spin_loop();
            }
        }
    }

    /// Reset the barrier for a new target
    pub fn reset(&self, num_cores: usize) {
        self.count.store(0, Ordering::Release);
        // Note: Can't change target in const context, would need interior mutability
    }
}

/// An atomic work counter for distributing work
pub struct WorkCounter {
    next: AtomicUsize,
    total: usize,
}

impl WorkCounter {
    pub const fn new(total: usize) -> Self {
        Self {
            next: AtomicUsize::new(0),
            total,
        }
    }

    /// Get the next work item index, returns None when all work is done
    pub fn get_next(&self) -> Option<usize> {
        let idx = self.next.fetch_add(1, Ordering::Relaxed);
        if idx < self.total {
            Some(idx)
        } else {
            None
        }
    }

    /// Reset the counter for new work
    pub fn reset(&self) {
        self.next.store(0, Ordering::Release);
    }

    /// Check if all work is done
    pub fn is_done(&self) -> bool {
        self.next.load(Ordering::Acquire) >= self.total
    }
}

/// Global barriers for frame synchronization
pub static RENDER_BARRIER: CoreBarrier = CoreBarrier::new(3); // 3 render cores
pub static FRAME_BARRIER: CoreBarrier = CoreBarrier::new(4); // All cores except network

//! Benchmark Application
//!
//! Performance benchmarking for graphics and game systems.

#![no_std]

/// Benchmark configuration
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    /// Screen width
    pub width: u32,
    /// Screen height
    pub height: u32,
    /// Duration in seconds
    pub duration: u32,
    /// Which benchmark to run
    pub benchmark_type: BenchmarkType,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            width: 1024,
            height: 768,
            duration: 30,
            benchmark_type: BenchmarkType::Rendering,
        }
    }
}

/// Types of benchmarks
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BenchmarkType {
    /// Rendering performance
    Rendering,
    /// Physics/collision performance
    Physics,
    /// Network throughput
    Network,
    /// Memory allocation stress
    Memory,
    /// Full game simulation
    FullGame,
}

/// Benchmark results
#[derive(Debug, Clone, Default)]
pub struct BenchmarkResults {
    /// Total frames rendered
    pub total_frames: u64,
    /// Average FPS
    pub avg_fps: f32,
    /// Minimum FPS
    pub min_fps: f32,
    /// Maximum FPS
    pub max_fps: f32,
    /// 1% low FPS
    pub low_1_percent: f32,
    /// Total triangles rendered
    pub total_triangles: u64,
    /// Average triangles per frame
    pub avg_triangles: u64,
}

/// Benchmark runner
pub struct Benchmark {
    config: BenchmarkConfig,
    results: BenchmarkResults,
    running: bool,
    frame_count: u64,
    elapsed_time: f32,
    frame_times: [f32; 256],
    frame_time_index: usize,
}

impl Benchmark {
    pub fn new(config: BenchmarkConfig) -> Self {
        Self {
            config,
            results: BenchmarkResults::default(),
            running: false,
            frame_count: 0,
            elapsed_time: 0.0,
            frame_times: [0.0; 256],
            frame_time_index: 0,
        }
    }

    /// Start the benchmark
    pub fn start(&mut self) {
        self.running = true;
        self.frame_count = 0;
        self.elapsed_time = 0.0;
        self.results = BenchmarkResults::default();
        self.frame_times = [0.0; 256];
        self.frame_time_index = 0;
    }

    /// Stop the benchmark and compute results
    pub fn stop(&mut self) -> BenchmarkResults {
        self.running = false;
        self.compute_results();
        self.results.clone()
    }

    /// Record a frame
    pub fn record_frame(&mut self, frame_time: f32, triangles: u64) {
        if !self.running {
            return;
        }

        self.frame_count += 1;
        self.elapsed_time += frame_time;
        self.results.total_triangles += triangles;

        // Store frame time for percentile calculations
        self.frame_times[self.frame_time_index] = frame_time;
        self.frame_time_index = (self.frame_time_index + 1) % 256;

        // Check if benchmark duration is reached
        if self.elapsed_time >= self.config.duration as f32 {
            self.stop();
        }
    }

    /// Check if benchmark is running
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Get progress (0.0 - 1.0)
    pub fn progress(&self) -> f32 {
        (self.elapsed_time / self.config.duration as f32).min(1.0)
    }

    /// Compute final results
    fn compute_results(&mut self) {
        self.results.total_frames = self.frame_count;

        if self.elapsed_time > 0.0 {
            self.results.avg_fps = self.frame_count as f32 / self.elapsed_time;
        }

        if self.frame_count > 0 {
            self.results.avg_triangles = self.results.total_triangles / self.frame_count;
        }

        // Compute min/max/percentile FPS from frame times
        let mut valid_times: [f32; 256] = [0.0; 256];
        let valid_count = self.frame_count.min(256) as usize;

        for i in 0..valid_count {
            valid_times[i] = self.frame_times[i];
        }

        if valid_count > 0 {
            // Sort frame times (simple insertion sort for small array)
            for i in 1..valid_count {
                let key = valid_times[i];
                let mut j = i;
                while j > 0 && valid_times[j - 1] > key {
                    valid_times[j] = valid_times[j - 1];
                    j -= 1;
                }
                valid_times[j] = key;
            }

            // Min FPS = 1 / max frame time
            let max_frame_time = valid_times[valid_count - 1];
            if max_frame_time > 0.0 {
                self.results.min_fps = 1.0 / max_frame_time;
            }

            // Max FPS = 1 / min frame time
            let min_frame_time = valid_times[0];
            if min_frame_time > 0.0 {
                self.results.max_fps = 1.0 / min_frame_time;
            }

            // 1% low = 1 / 99th percentile frame time
            let percentile_idx = (valid_count * 99) / 100;
            let percentile_time = valid_times[percentile_idx.min(valid_count - 1)];
            if percentile_time > 0.0 {
                self.results.low_1_percent = 1.0 / percentile_time;
            }
        }
    }

    /// Get current results (partial while running)
    pub fn results(&self) -> &BenchmarkResults {
        &self.results
    }

    /// Get config
    pub fn config(&self) -> &BenchmarkConfig {
        &self.config
    }
}

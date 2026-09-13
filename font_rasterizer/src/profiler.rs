use std::path::Path;
use wgpu_profiler::{GpuProfiler, GpuProfilerSettings, GpuTimerQueryResult};

#[derive(Debug, Clone)]
pub struct CpuPassTimer {
    pub label: String,
    pub duration_sec: f64,
}

pub struct Profiler {
    profiler: Option<GpuProfiler>,
    enabled: bool,
    latest_results: Vec<GpuTimerQueryResult>,
    history: Vec<Vec<GpuTimerQueryResult>>,
    cpu_history: Vec<Vec<CpuPassTimer>>,
    current_frame_cpu_timers: Vec<CpuPassTimer>,
}

impl Profiler {
    pub fn new(device: &wgpu::Device) -> Self {
        let settings = GpuProfilerSettings {
            enable_timer_queries: true,
            enable_debug_groups: true,
            ..Default::default()
        };
        log::info!("GpuProfilerSettings: {:?}", settings);
        match GpuProfiler::new(device, settings) {
            Ok(profiler) => {
                log::info!("GpuProfiler successfully initialized with TIMESTAMP_QUERY support.");
                Self {
                    profiler: Some(profiler),
                    enabled: true,
                    latest_results: Vec::new(),
                    history: Vec::new(),
                    cpu_history: Vec::new(),
                    current_frame_cpu_timers: Vec::new(),
                }
            }
            Err(err) => {
                log::warn!("Failed to initialize GpuProfiler: {:?}", err);
                Self {
                    profiler: None,
                    enabled: true,
                    latest_results: Vec::new(),
                    history: Vec::new(),
                    cpu_history: Vec::new(),
                    current_frame_cpu_timers: Vec::new(),
                }
            }
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// クロージャ内で CommandEncoder を操作し、その範囲をプロファイリングスコープとする
    pub fn scope_fn<F, R>(&mut self, name: &str, encoder: &mut wgpu::CommandEncoder, f: F) -> R
    where
        F: FnOnce(&mut wgpu::CommandEncoder) -> R,
    {
        if self.enabled {
            let start = web_time::Instant::now();
            let result = if let Some(ref mut profiler) = self.profiler {
                let mut scope = profiler.scope(name, encoder);
                f(&mut scope)
            } else {
                f(encoder)
            };
            let elapsed = start.elapsed().as_secs_f64();
            self.current_frame_cpu_timers.push(CpuPassTimer {
                label: name.to_string(),
                duration_sec: elapsed,
            });
            result
        } else {
            f(encoder)
        }
    }

    /// フレームのエンコード終了時にタイムスタンプクエリを解決する
    pub fn resolve_queries(&mut self, encoder: &mut wgpu::CommandEncoder) {
        if self.enabled
            && let Some(ref mut profiler) = self.profiler
        {
            profiler.resolve_queries(encoder);
        }
    }

    /// フレームのエンコード終了時にプロファイラにフレーム完了を通知する（queue.submit の直前に呼び出す）
    pub fn end_frame(&mut self) {
        if self.enabled {
            if let Some(ref mut profiler) = self.profiler {
                let _ = profiler.end_frame();
            }
            let cpu_timers = std::mem::take(&mut self.current_frame_cpu_timers);
            self.cpu_history.push(cpu_timers);
        }
    }

    /// queue.submit 後に呼び出し、準備のできたプロファイリング結果を処理・回収する
    pub fn process_finished_frame(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        if self.enabled
            && let Some(ref mut profiler) = self.profiler
        {
            let period = queue.get_timestamp_period();
            let first_try = profiler.process_finished_frame(period);
            if let Some(results) = first_try {
                self.latest_results = results.clone();
                self.history.push(results);
            } else {
                let _ = device.poll(wgpu::wgt::PollType::Wait {
                    submission_index: None,
                    timeout: None,
                });
                if let Some(results) = profiler.process_finished_frame(period) {
                    self.latest_results = results.clone();
                    self.history.push(results);
                }
            }
        }
    }

    /// 残りのフレームのプロファイル結果をすべてフラッシュ（処理完了まで待機して回収）する
    pub fn flush(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        if self.enabled
            && let Some(ref mut profiler) = self.profiler
        {
            let period = queue.get_timestamp_period();
            for _ in 0..10 {
                let res = profiler.process_finished_frame(period);
                let _ = device.poll(wgpu::wgt::PollType::Wait {
                    submission_index: None,
                    timeout: None,
                });
                if let Some(results) = res {
                    self.latest_results = results.clone();
                    self.history.push(results);
                }
            }
        }
    }

    pub fn latest_results(&self) -> &[GpuTimerQueryResult] {
        &self.latest_results
    }

    pub fn print_report(&self) {
        if self.latest_results.is_empty() {
            println!("No GPU profiling results available.");
            return;
        }

        println!("\n============================================================");
        println!("  WGSL / GPU Profiling Report (Latest Frame)");
        println!("============================================================");
        print_query_results(&self.latest_results, 0);
        println!("============================================================\n");
    }

    pub fn print_summary(&self) {
        let total_frames = self.history.len().max(self.cpu_history.len());
        if total_frames == 0 {
            println!("No profiling history available.");
            return;
        }

        let mut gpu_pass_stats: std::collections::BTreeMap<String, Vec<f64>> =
            std::collections::BTreeMap::new();

        for frame_results in &self.history {
            collect_durations(frame_results, &mut gpu_pass_stats, "");
        }

        let has_gpu_data = gpu_pass_stats.values().any(|v| !v.is_empty());

        println!("\n============================================================");
        if has_gpu_data {
            println!("  WGSL / GPU Profiling Summary ({} frames)", total_frames);
            println!("============================================================");
            for (name, times) in &gpu_pass_stats {
                let count = times.len();
                if count == 0 {
                    continue;
                }
                let sum: f64 = times.iter().sum();
                let avg = sum / count as f64;
                let min = times.iter().copied().fold(f64::INFINITY, f64::min);
                let max = times.iter().copied().fold(f64::NEG_INFINITY, f64::max);

                println!(
                    "  [GPU] {:<25}: Avg {:7.3} ms | Min {:7.3} ms | Max {:7.3} ms ({})",
                    name,
                    avg * 1000.0,
                    min * 1000.0,
                    max * 1000.0,
                    count
                );
            }
        } else {
            println!(
                "  WGSL / Pass Profiling Summary (CPU Stage Time) ({} frames)",
                total_frames
            );
            println!("============================================================");

            let mut cpu_pass_stats: std::collections::BTreeMap<String, Vec<f64>> =
                std::collections::BTreeMap::new();

            for frame_timers in &self.cpu_history {
                for timer in frame_timers {
                    cpu_pass_stats
                        .entry(timer.label.clone())
                        .or_default()
                        .push(timer.duration_sec);
                }
            }

            for (name, times) in &cpu_pass_stats {
                let count = times.len();
                if count == 0 {
                    continue;
                }
                let sum: f64 = times.iter().sum();
                let avg = sum / count as f64;
                let min = times.iter().copied().fold(f64::INFINITY, f64::min);
                let max = times.iter().copied().fold(f64::NEG_INFINITY, f64::max);

                println!(
                    "  [Pass] {:<24}: Avg {:7.3} ms | Min {:7.3} ms | Max {:7.3} ms ({})",
                    name,
                    avg * 1000.0,
                    min * 1000.0,
                    max * 1000.0,
                    count
                );
            }
        }
        println!("============================================================\n");
    }

    pub fn write_chrometrace(&self, path: &Path) -> Result<(), std::io::Error> {
        if let Some(last) = self.history.last() {
            wgpu_profiler::chrometrace::write_chrometrace(path, last)?;
        }
        Ok(())
    }
}

fn print_query_results(results: &[GpuTimerQueryResult], indent: usize) {
    let indent_str = " ".repeat(indent * 2);
    for res in results {
        let time_str = if let Some(time_range) = &res.time {
            let duration_ms = (time_range.end - time_range.start) * 1000.0;
            format!("{:.3} ms", duration_ms)
        } else {
            "N/A".to_string()
        };
        println!("  {}{:<30}: {}", indent_str, res.label, time_str);
        print_query_results(&res.nested_queries, indent + 1);
    }
}

fn collect_durations(
    results: &[GpuTimerQueryResult],
    stats: &mut std::collections::BTreeMap<String, Vec<f64>>,
    prefix: &str,
) {
    for res in results {
        let name = if prefix.is_empty() {
            res.label.clone()
        } else {
            format!("{}/{}", prefix, res.label)
        };
        if let Some(time_range) = &res.time {
            let duration_sec = time_range.end - time_range.start;
            stats.entry(name.clone()).or_default().push(duration_sec);
        }
        collect_durations(&res.nested_queries, stats, &name);
    }
}

use std::{collections::BTreeMap, sync::Mutex};

use instant::{Duration, Instant};
use once_cell::sync::Lazy;

static METRICS_COUNTER: Lazy<Mutex<MetricsCounter>> = Lazy::new(Default::default);

#[inline]
pub(super) fn record_start_of_phase(phase_name: &str) {
    METRICS_COUNTER.lock().unwrap().start_phase(phase_name);
}

#[inline]
pub(super) fn print_metrics_to_stdout() {
    println!("{}", METRICS_COUNTER.lock().unwrap().to_string());
}

#[derive(Default)]
struct MetricsCounter {
    current_phase: Option<Phase>,
    phase_times: BTreeMap<String, PhaseStats>,
}

struct Phase {
    name: String,
    start_time: Instant,
}

struct PhaseStats {
    count: u64,
    total_time: Duration,
    max_time: Duration,
    min_time: Duration,
}

impl MetricsCounter {
    #[inline]
    fn start_phase(&mut self, phase_name: &str) {
        let current_time = Instant::now();
        if let Some(phase) = &self.current_phase {
            let phase_time = current_time - phase.start_time;
            let stats = self
                .phase_times
                .entry(phase.name.clone())
                .or_insert(PhaseStats {
                    count: 0,
                    total_time: Duration::ZERO,
                    max_time: Duration::ZERO,
                    min_time: Duration::MAX,
                });
            stats.count += 1;
            stats.total_time += phase_time;
            stats.max_time = stats.max_time.max(phase_time);
            stats.min_time = stats.min_time.min(phase_time);
        }
        self.current_phase = Some(Phase {
            name: phase_name.to_string(),
            start_time: current_time,
        });
    }
}

impl ToString for MetricsCounter {
    fn to_string(&self) -> String {
        let mut result = String::new();
        for (phase_name, stats) in &self.phase_times {
            result.push_str(&format!(
                "Phase: {}\tcount: {}\ttotal:{}\tavg: {}ms\tmax: {}ms\tmin: {}ms\n",
                phase_name,
                stats.count,
                stats.total_time.as_millis(),
                stats.total_time.as_millis() / stats.count as u128,
                stats.max_time.as_millis(),
                stats.min_time.as_millis(),
            ));
        }
        result
    }
}

use std::collections::HashMap;

use instant::Instant;

#[derive(Default)]
pub(super) struct MetricsCounter {
    current_phase: Option<Phase>,
    phase_times: HashMap<String, PhaseStats>,
}

struct Phase {
    name: String,
    start_time: u128,
}

struct PhaseStats {
    count: u64,
    total_time: u128,
    max_time: u128,
    min_time: u128,
}

impl MetricsCounter {
    pub(super) fn start_phase(&mut self, phase_name: &str) {
        let current_time = Instant::now().elapsed().as_nanos();
        if let Some(phase) = &self.current_phase {
            let phase_time = current_time.saturating_sub(phase.start_time);
            let stats = self
                .phase_times
                .entry(phase.name.clone())
                .or_insert(PhaseStats {
                    count: 0,
                    total_time: 0,
                    max_time: 0,
                    min_time: std::u128::MAX,
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
                "Phase: {}\tcount: {}\ttotal time:{}\taverage time: {}ms\tmax time: {}ms\tmin time: {}ms\n",
                phase_name,
                stats.count,
                stats.total_time,
                stats.total_time as f64 / stats.count as f64 / 1000000.0,
                stats.max_time as f64 / 1000000.0,
                stats.min_time as f64 / 1000000.0,
            ));
        }
        result
    }
}

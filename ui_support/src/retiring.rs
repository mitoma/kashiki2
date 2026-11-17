use font_rasterizer::time::now_millis;
use web_time::Duration;

use crate::easing_value::EasingPointN;

/// 不透明度を 1 から 0 へ下げる退場演出。終わる時刻を明示的に持つ
pub struct FadeOut {
    pub(crate) opacity: EasingPointN<1>,
    deadline: u32,
}

impl FadeOut {
    pub fn start(duration: Duration) -> Self {
        let mut opacity = EasingPointN::new([1.0]);
        opacity.update_duration_and_easing_func(duration, nenobi::functions::sin_in_out);
        opacity.update([0.0]);
        Self {
            opacity,
            deadline: now_millis().saturating_add(duration.as_millis() as u32),
        }
    }

    pub fn opacity(&self) -> f32 {
        let [opacity] = self.opacity.current();
        opacity
    }

    pub fn deadline(&self) -> u32 {
        self.deadline
    }

    pub fn finished(&self) -> bool {
        now_millis() >= self.deadline
    }
}

/// 退場中の要素を一意に識別する ID
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct RetireId(usize);

impl RetireId {
    pub(crate) fn value(&self) -> usize {
        self.0
    }
}

struct RetiringEntry<T> {
    id: RetireId,
    deadline: u32,
    value: T,
}

/// 削除アニメーション中の要素を保持し、破棄時刻(deadline)を過ぎたら取り除く
pub(crate) struct Retiring<T> {
    entries: Vec<RetiringEntry<T>>,
    next_id: usize,
}

impl<T> Default for Retiring<T> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            next_id: 0,
        }
    }
}

impl<T> Retiring<T> {
    pub(crate) fn retire(&mut self, value: T, deadline: u32) -> RetireId {
        let id = RetireId(self.next_id);
        self.next_id += 1;
        self.entries.push(RetiringEntry {
            id,
            deadline,
            value,
        });
        id
    }

    pub(crate) fn sweep(&mut self, now: u32, mut on_drop: impl FnMut(RetireId, T)) {
        let mut index = 0;
        while index < self.entries.len() {
            if self.entries[index].deadline <= now {
                let entry = self.entries.swap_remove(index);
                on_drop(entry.id, entry.value);
            } else {
                index += 1;
            }
        }
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &T> {
        self.entries.iter().map(|entry| &entry.value)
    }

    pub(crate) fn iter_mut(&mut self) -> impl Iterator<Item = (RetireId, &mut T)> {
        self.entries
            .iter_mut()
            .map(|entry| (entry.id, &mut entry.value))
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }
}

/// GPU モーションと CPU イージングの両方が終わる時刻を破棄時刻とする
pub(crate) fn retire_deadline(
    now: u32,
    gpu_start_time: u32,
    gpu_duration: Duration,
    cpu_duration: Duration,
) -> u32 {
    let gpu_end = gpu_start_time.saturating_add(gpu_duration.as_millis() as u32);
    let cpu_end = now.saturating_add(cpu_duration.as_millis() as u32);
    gpu_end.max(cpu_end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sweep_drops_only_entries_past_deadline() {
        let mut retiring = Retiring::default();
        let early = retiring.retire("early", 100);
        let late = retiring.retire("late", 200);

        let mut dropped = Vec::new();
        retiring.sweep(150, |id, value| dropped.push((id, value)));
        assert_eq!(dropped, vec![(early, "early")]);
        assert_eq!(retiring.len(), 1);

        dropped.clear();
        retiring.sweep(200, |id, value| dropped.push((id, value)));
        assert_eq!(dropped, vec![(late, "late")]);
        assert_eq!(retiring.len(), 0);
    }

    #[test]
    fn retire_assigns_unique_ids_for_same_value() {
        let mut retiring = Retiring::default();
        let first = retiring.retire('a', 100);
        let second = retiring.retire('a', 100);
        assert_ne!(first, second);
        assert_eq!(retiring.len(), 2);
    }

    #[test]
    fn deadline_waits_for_delayed_gpu_motion() {
        // 開始が遅延された GPU モーションは CPU イージングより後に終わる
        let deadline = retire_deadline(
            1_000,
            1_600,
            Duration::from_millis(500),
            Duration::from_millis(500),
        );
        assert_eq!(deadline, 2_100);
    }

    #[test]
    fn deadline_waits_for_cpu_easing() {
        let deadline = retire_deadline(
            1_000,
            1_000,
            Duration::from_millis(300),
            Duration::from_millis(800),
        );
        assert_eq!(deadline, 1_800);
    }
}

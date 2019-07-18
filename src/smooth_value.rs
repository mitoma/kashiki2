use std::collections::*;
use std::f64::*;

pub enum MovingType {
    Liner,
    Smooth,
    SmoothIn,
    SmoothOut,
    Bound,
}

impl MovingType {
    fn gains(&self, num_of_div: i16) -> Vec<f64> {
        let mut vec = Vec::new();
        match self {
            MovingType::Liner => {
                let gain = 1.0 / num_of_div as f64;
                for _ in 0..num_of_div {
                    vec.push(gain)
                }
            }
            MovingType::Smooth => {
                let gain = 1.0 / num_of_div as f64;
                let g = consts::PI * gain;
                for i in 0..num_of_div {
                    let i = i as f64;
                    let div_gain = (g * i).cos() - (g * (i + 1.0)).cos();
                    vec.push(div_gain / 2.0)
                }
            }
            MovingType::SmoothOut => {
                let gain = 1.0 / num_of_div as f64;
                let g = consts::PI * gain / 2.0;
                for i in 0..num_of_div {
                    let i = i as f64;
                    let div_gain = (g * (i + 1.0)).sin() - (g * i).sin();
                    vec.push(div_gain)
                }
            }
            MovingType::SmoothIn => {
                let gain = 1.0 / num_of_div as f64;
                let g = consts::PI * gain / 2.0;
                let start = consts::PI * 1.5;
                for i in 0..num_of_div {
                    let i = i as f64;
                    let div_gain = (start + (g * (i + 1.0))).sin() - (start + (g * i)).sin();
                    vec.push(div_gain)
                }
            }
            MovingType::Bound => {
                // TODO 壊れてる
                let gain = 1.0 / num_of_div as f64;
                let g = consts::PI * 1.5 * gain;
                let qg = consts::PI * 4.0;
                let dd = qg.sin() * 2.0;
                for i in 0..num_of_div {
                    let i = i as f64;
                    let div_gain = ((g * i) + qg).sin() - (g * (i + 1.0) + qg).sin();
                    vec.push(div_gain / dd)
                }
            }
        }
        vec
    }
}

pub struct SmoothValue {
    value: f64,
    current_value: f64,
    moving_type: MovingType,
    num_of_div: i16,
    queue: VecDeque<f64>,
}

impl SmoothValue {
    pub fn new(value: f64, moving_type: MovingType, num_of_div: i16) -> SmoothValue {
        SmoothValue {
            value: value,
            current_value: value,
            moving_type: moving_type,
            num_of_div: num_of_div,
            queue: VecDeque::new(),
        }
    }

    pub fn next(&mut self) -> f64 {
        if let Some(gain_value) = self.queue.pop_front() {
            self.current_value += gain_value;
        }
        self.current_value
    }

    pub fn update(&mut self, next_value: f64) {
        if self.value == next_value {
            return;
        }
        let mut new_queue: VecDeque<f64> = VecDeque::new();
        let delta = next_value - self.value;
        self.value = next_value;

        let gains = self.moving_type.gains(self.num_of_div);
        gains.iter().for_each(|gain| {
            let mut value = gain * delta;
            if let Some(v) = self.queue.pop_front() {
                value += v;
            }
            new_queue.push_back(value)
        });
        self.queue = new_queue
    }

    pub fn add(&mut self, add_value: f64) {
        self.update(self.value + add_value);
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn liner() {
        let mut value = SmoothValue::new(100.0, MovingType::Liner, 5);
        value.update(200.0);
        assert_eq!(value.next(), 120.0);
        assert_eq!(value.next(), 140.0);
        assert_eq!(value.next(), 160.0);
        assert_eq!(value.next(), 180.0);
        assert_eq!(value.next(), 200.0);
        assert_eq!(value.next(), 200.0);
        value.update(100.0);
        assert_eq!(value.next(), 180.0);
        assert_eq!(value.next(), 160.0);
        value.update(0.0);
        assert_eq!(value.next(), 120.0);
        assert_eq!(value.next(), 80.0);
        assert_eq!(value.next(), 40.0);
        assert_eq!(value.next(), 20.0);
        assert_eq!(value.next(), 0.0);
        assert_eq!(value.next(), 0.0);
    }

}

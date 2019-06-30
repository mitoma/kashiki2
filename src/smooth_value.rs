use std::collections::*;

pub enum MovingType {
    Liner,
    Smooth,
    SmoothIn,
    SmoothOut,
}

struct SmoothValue {
    value: f64,
    current_value: f64,
    moving_type: MovingType,
    num_of_div: i16,
    queue: VecDeque<f64>,
}

impl SmoothValue {
    pub fn new(
        value: f64,
        current_value: f64,
        moving_type: MovingType,
        num_of_div: i16,
    ) -> SmoothValue {
        SmoothValue {
            value: value,
            current_value: value,
            moving_type: moving_type,
            num_of_div: num_of_div,
            queue: VecDeque::new(),
        }
    }

    pub fn next(&mut self) -> &SmoothValue {
        if let Some(next_value) = self.queue.pop_front() {
            self.current_value = next_value;
        }
        self
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn liner() {
        let value = SmoothValue::new(100.0, 100.0, MovingType::Liner, 15);
        value.current_value;
    }

}
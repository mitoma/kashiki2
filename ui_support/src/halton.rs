use cached::proc_macro::cached;

// Halton Sequence
#[cached]
pub(crate) fn halton_sequence(base: u32, index: u32) -> f32 {
    let mut result = 0.0;
    let mut f = 1.0;
    let mut i = index;

    while i > 0 {
        f /= base as f32;
        result += f * (i % base) as f32;
        i /= base;
    }

    result
}

#[cfg(test)]
mod tests {
    use crate::halton::halton_sequence;

    #[test]
    fn test_halton_sequence() {
        assert_eq!(halton_sequence(2, 0), 0.0);
        assert_eq!(halton_sequence(2, 1), 0.5);
        assert_eq!(halton_sequence(2, 2), 0.25);
        assert_eq!(halton_sequence(3, 0), 0.0);
        assert_eq!(halton_sequence(3, 1), 1.0 / 3.0);
        assert_eq!(halton_sequence(3, 2), 2.0 / 3.0);
    }
}

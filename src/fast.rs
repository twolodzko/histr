use crate::hist::StreamHist;

// The faster but less precise alternatives of the `cdf` and `quantile` algorithms
// that do not use any interpolation.

impl StreamHist {
    /// This is a computationally cheaper but less precise alternative to [`StreamHist::count_by`]
    /// that doesn't use interpolation.
    ///
    /// # NaN propagation
    ///
    /// If the `value` is `f64::NAN`, it will return `f64::NAN`.
    pub fn fast_count_by(&self, value: f64) -> f64 {
        if value.is_nan() {
            return f64::NAN;
        }
        if self.is_empty() || value <= self.min {
            return 0.0;
        }
        if value > self.max {
            return self.count();
        }
        self.iter()
            .take_while(|bin| bin.mean <= value)
            .fold(0, |acc, x| acc + x.count) as f64
    }

    /// This is a computationally cheaper but less precise alternative to [`StreamHist::cdf`]
    /// that doesn't use interpolation.
    ///
    /// # NaN propagation
    ///
    /// If the `value` is `f64::NAN`, it will return `f64::NAN`.
    pub fn fast_cdf(&self, value: f64) -> f64 {
        self.fast_count_by(value) / self.count()
    }

    /// This is a computationally cheaper but less precise alternative to [`StreamHist::quantile`]
    /// that doesn't use interpolation.
    ///
    /// # Panics
    ///
    /// `prob` needs to be a probability value between `0.0` and `1.0` (inclusive),
    /// otherwise it panics.
    pub fn fast_quantile(&self, prob: f64) -> f64 {
        assert!(
            (0.0..=1.0).contains(&prob),
            "{prob} is not a valid probability"
        );

        let count = prob * self.count();
        self.iter()
            .scan((0.0, self.min), |(acc, _), bin| {
                *acc += bin.count as f64;
                Some((*acc, bin.mean))
            })
            .take_while(|(acc, _)| acc <= &count)
            .last()
            .map_or(self.min, |(_, mean)| mean)
    }
}

#[cfg(test)]
mod tests {
    use crate::hist::StreamHist;
    use test_case::test_case;

    #[test_case(f64::NAN ; "NaN")]
    #[test_case(f64::INFINITY ; "infinity")]
    #[test_case(f64::NEG_INFINITY ; "negative infinity")]
    #[test_case(-1.0 ; "negative")]
    #[test_case(2.0 ; "too large")]
    #[should_panic]
    fn fast_quantile_prob_invalid(value: f64) {
        StreamHist::default().fast_quantile(value);
    }
}

use crate::hist::StreamHist;

// The faster but less precise alternatives of the `cdf` and `quantile` algorithms
// that do not use any interpolation.

impl StreamHist {
    /// This is a computationally cheaper but less precise alternative to [`StreamHist::count_by`]
    /// that doesn't use interpolation.
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
    pub fn fast_cdf(&self, value: f64) -> f64 {
        self.fast_count_by(value) / self.count()
    }

    /// This is a computationally cheaper but less precise alternative to [`StreamHist::quantile`]
    /// that doesn't use interpolation.
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

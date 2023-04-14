use crate::bin::{sum_counts, Bin};
use crate::hist::StreamHist;

impl StreamHist {
    /// Approximate mean of the data.
    ///
    /// Calculates [weighted mean] of the bins weighting their means by the counts.
    ///
    /// [weighted mean]: https://en.wikipedia.org/wiki/Weighted_arithmetic_mean
    ///
    /// # Examples
    ///
    /// ```
    /// use streamhist::StreamHist;
    /// use streamhist::Bin;
    ///
    /// // Using this data example: https://www.investopedia.com/terms/w/weightedaverage.asp
    /// let hist = StreamHist::from(vec![Bin::new(10.0, 2), Bin::new(50.0, 5), Bin::new(40.0, 3)]);
    ///
    /// assert_eq!(hist.mean(), 39.0);
    /// ```
    pub fn mean(&self) -> f64 {
        if self.is_empty() {
            return f64::NAN;
        }
        self.iter()
            .fold(0.0, |acc, x| acc + x.mean * x.count as f64)
            / self.count()
    }

    /// Approximate variance of the data.
    ///
    /// Calculates [weighted variance] of the bins weighting them by their counts.
    ///
    /// [weighted variance]: https://en.wikipedia.org/wiki/Weighted_arithmetic_mean#Weighted_sample_variance
    ///
    /// # Examples
    ///
    /// ```
    /// use streamhist::StreamHist;
    /// use streamhist::Bin;
    ///
    /// // Using this data example: https://www.investopedia.com/terms/w/weightedaverage.asp
    /// let hist = StreamHist::from(vec![Bin::new(10.0, 2), Bin::new(50.0, 5), Bin::new(40.0, 3)]);
    ///
    /// assert_eq!(hist.variance(), 229.0);
    /// ```
    pub fn variance(&self) -> f64 {
        if self.is_empty() {
            return f64::NAN;
        }
        let m = self.mean();
        self.iter()
            .fold(0.0, |acc, x| acc + x.count as f64 * (x.mean - m).powi(2))
            / self.count()
    }

    /// Standard deviation of the data.
    ///
    /// Square root of the [`StreamHist::variance`].
    pub fn stdev(&self) -> f64 {
        self.variance().sqrt()
    }

    /// Approximate count of the number of values since the `value`.
    ///
    /// If `value` is a `f64::NAN`, the NaN value will be returned.
    ///
    /// It uses the "sum" procedure described by Ben-Haim and Tom-Tov (2010).
    pub fn count_by(&self, value: f64) -> f64 {
        if value.is_nan() {
            return f64::NAN;
        }
        if self.is_empty() || value <= self.min {
            return 0.0;
        }
        if value > self.max {
            return self.count();
        }

        // Algorithm 3: Sum Procedure from Ben-Haim & Tom-Tov (2010), p. 852
        let idx = self.partition_point(value);
        let sum = sum_counts(&self.bins[..idx.saturating_sub(1)]) as f64;

        let (left, right) = self.neighbors(idx);
        let (pi, mi) = (left.mean, left.count as f64);
        let (pj, mj) = (right.mean, right.count as f64);

        let s = if pj - pi <= 0.0 {
            0.0
        } else {
            let mb = mi + (mj - mi) / (pj - pi) * (value - pi);
            (mi + mb) / 2.0 * (value - pi) / (pj - pi)
        };
        sum + mi / 2.0 + s
    }

    /// Approximate empirical cumulative distribution function of the data for a given `value`.
    ///
    /// The result of [`StreamHist::count_by`] divided by the total [`StreamHist::count`].
    ///
    /// # Examples
    ///
    /// ```
    /// use streamhist::StreamHist;
    ///
    /// let hist = StreamHist::from(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
    /// assert_eq!(hist.cdf(3.0), 0.5);
    /// ```
    pub fn cdf(&self, value: f64) -> f64 {
        self.count_by(value) / self.count()
    }

    /// Approximate sample quantile of the data for a given probability `prob`.
    ///
    /// The `prob` argument needs to be between 0.0 and 1.0, otherwise it will panic.
    ///
    /// It uses the "uniform" procedure described by Ben-Haim and Tom-Tov (2010).
    ///
    /// # Examples
    ///
    /// ```
    /// use streamhist::StreamHist;
    ///
    /// let hist = StreamHist::from(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
    /// assert_eq!(hist.quantile(0.5), 3.0);
    /// ```
    pub fn quantile(&self, prob: f64) -> f64 {
        assert!(
            (0.0..=1.0).contains(&prob),
            "{prob} is not a valid probability"
        );
        if self.is_empty() {
            return f64::NAN;
        }
        if prob == 0.0 {
            return self.min;
        }
        if prob == 1.0 {
            return self.max;
        }

        // Algorithm 4: Uniform Procedure from Ben-Haim & Tom-Tov (2010), p. 853
        let count = prob * self.count();
        let (idx, sum) = self.find_cumulative_count_by(count);

        let (left, right) = self.neighbors(idx);
        let (pi, mi) = (left.mean, left.count as f64);
        let (pj, mj) = (right.mean, right.count as f64);

        let d = count - sum;
        let a = mj - mi;
        if a == 0.0 {
            return pi + (pj - pi) * (d / mi);
        }
        let b = 2.0 * mi;
        let c = -2.0 * d;
        let z = (-b + (b.powi(2) - 4.0 * a * c).sqrt()) / (2.0 * a);
        pi + (pj - pi) * z
    }

    /// Find an index of the cumulative sum of counts, return the index and the sum.
    fn find_cumulative_count_by(&self, value: f64) -> (usize, f64) {
        debug_assert!(!value.is_nan());
        let mut idx = 0;
        let mut sum = 0.0;
        let mut prev = 0.0;
        for bin in self.iter() {
            let this = bin.count as f64 / 2.0;
            // compare to the midpoint between the two bins
            if sum + this + prev > value {
                break;
            }
            sum += prev + this;
            prev = this;
            idx += 1;
        }
        (idx, sum)
    }

    /// Returns the bins at indexes `index-1` and `index`.
    #[inline]
    fn neighbors(&self, index: usize) -> (Bin, Bin) {
        if index == 0 {
            let first = Bin::new(self.min, 0);
            (first, self.bins.first().cloned().unwrap_or(first))
        } else if index >= self.bins.len() {
            let last = Bin::new(self.max, 0);
            (self.bins.last().cloned().unwrap_or(last), last)
        } else {
            (self.bins[index - 1], self.bins[index])
        }
    }

    /// Approximate median of the data.
    ///
    /// The [`StreamHist::quantile`] evaluated at 0.5.
    ///
    /// # Examples
    ///
    /// ```
    /// use streamhist::StreamHist;
    ///
    /// let hist = StreamHist::from(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
    /// assert_eq!(hist.median(), 3.0);
    /// ```
    pub fn median(&self) -> f64 {
        self.quantile(0.5)
    }
}

#[cfg(test)]
mod tests {
    use crate::bin::Bin;
    use crate::hist::StreamHist;
    use test_case::test_case;

    #[test]
    fn cdf() {
        let hist = StreamHist::from(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        assert_eq!(hist.cdf(0.0), 0.0);
        assert_eq!(hist.cdf(3.0), 0.5);
        assert_eq!(hist.cdf(hist.max + 0.1), 1.0);

        assert_eq!(hist.cdf(f64::NEG_INFINITY), 0.0);
        assert_eq!(hist.cdf(f64::INFINITY), 1.0);
        assert!(hist.cdf(f64::NAN).is_nan());
    }

    #[test]
    fn count_by_nan() {
        let hist = StreamHist::from(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        assert_eq!(hist.count_by(f64::NEG_INFINITY), 0.0);
        assert_eq!(hist.count_by(f64::INFINITY), 5.0);
        assert!(hist.count_by(f64::NAN).is_nan());
    }

    #[test]
    fn count_by() {
        assert_eq!(StreamHist::with_capacity(5).count_by(2.0), 0.0);

        let hist = StreamHist::from(vec![0.0]);
        assert_eq!(hist.count_by(0.0), 0.0);
        assert_eq!(hist.count_by(-2.0), 0.0);
        assert_eq!(hist.count_by(2.0), 1.0);

        let hist = StreamHist::from(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        assert_eq!(hist.count_by(1.0), 0.0);
        assert_eq!(hist.count_by(0.0), 0.0);
        assert_eq!(hist.count_by(1.5), 1.0);
        assert_eq!(hist.count_by(2.0), 1.5);
        assert_eq!(hist.count_by(3.0), 2.5);
        assert_eq!(hist.count_by(4.0), 3.5);
        assert_eq!(hist.count_by(4.5), 4.0);
        assert_eq!(hist.count_by(5.0), 4.5);
        assert_eq!(hist.count_by(6.0), 5.0);

        let hist = StreamHist {
            bins: vec![Bin::new(7.0, 3), Bin::from(20.0), Bin::new(34.0, 3)],
            min: 1.0,
            max: 37.0,
            size: 3,
        };
        assert_eq!(hist.count_by(0.0), 0.0);
        assert_eq!(hist.count_by(40.0), 7.0);

        assert!((0.0..3.0).contains(&hist.count_by(2.0)));
        assert!((0.0..3.0).contains(&hist.count_by(5.0)));
        assert!((1.0..=3.0).contains(&hist.count_by(7.0)));
        assert!((1.0..4.0).contains(&hist.count_by(8.0)));
        assert!((3.0..4.0).contains(&hist.count_by(19.0)));
        assert!((3.0..=4.0).contains(&hist.count_by(20.0)));
        assert!((3.0..7.0).contains(&hist.count_by(21.0)));
        assert!((4.0..=7.0).contains(&hist.count_by(34.0)));
        assert!((4.0..=7.0).contains(&hist.count_by(36.0)));
        assert!((4.0..=7.0).contains(&hist.count_by(37.0)));
    }

    #[test]
    fn counts_are_monotonic() {
        let hist = StreamHist {
            bins: vec![Bin::new(7.0, 3), Bin::from(20.0), Bin::new(34.0, 3)],
            min: 1.0,
            max: 37.0,
            size: 3,
        };

        // The cumulative counts are monotonically increasing
        let mut value = 0.0;
        let mut prev_count = 0.0;
        while value < 40.0 {
            let count = hist.count_by(value);
            assert!(prev_count <= count);
            assert!(count >= 0.0);
            assert!(count <= hist.count());
            value += 0.01;
            prev_count = count;
        }
    }

    #[test_case(f64::NAN ; "NaN")]
    #[test_case(f64::INFINITY ; "infinity")]
    #[test_case(f64::INFINITY ; "negative infinity")]
    #[test_case(-1.0 ; "negative")]
    #[test_case(2.0 ; "too large")]
    #[should_panic]
    fn quantile_prob_invalid(value: f64) {
        StreamHist::default().quantile(value);
    }

    #[test]
    fn quantile() {
        let hist = StreamHist::from(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        assert_eq!(hist.quantile(0.0), 1.0);
        assert_eq!(hist.quantile(0.2), 1.5);
        assert_eq!(hist.quantile(0.5), 3.0);
        assert_eq!(hist.quantile(1.0), 5.0);
        assert_eq!(hist.median(), 3.0);

        assert!(StreamHist::with_capacity(10).quantile(0.0).is_nan());
        assert!(StreamHist::with_capacity(10).quantile(0.5).is_nan());
        assert!(StreamHist::with_capacity(10).median().is_nan());
    }

    #[test]
    fn quantiles_are_monotonic() {
        let hist = StreamHist {
            bins: vec![Bin::new(7.0, 3), Bin::from(20.0), Bin::new(34.0, 3)],
            min: 1.0,
            max: 37.0,
            size: 3,
        };
        // Quantiles are monotonically increasing
        let mut prob = 0.0;
        let mut prev_value = 0.0;
        while prob <= 1.0 {
            let value = hist.quantile(prob);
            assert!(!value.is_nan());
            assert!(value.is_finite());
            assert!(prev_value <= value);
            assert!(value >= hist.min);
            assert!(value <= hist.max);
            prob += 0.001;
            prev_value = value;
        }
    }

    #[test]
    fn mean() {
        assert!(StreamHist::with_capacity(10).mean().is_nan());
        assert_eq!(StreamHist::from(vec![1.0, 2.0]).mean(), 1.5);
        assert_eq!(
            StreamHist::from(vec![Bin::from(1.0), Bin::new(2.0, 2), Bin::from(3.0)]).mean(),
            2.0
        );
        assert_eq!(StreamHist::from(vec![1.0, 2.0, 3.0, 4.0, 5.0]).mean(), 3.0);
        assert_eq!(
            StreamHist::from(vec![
                Bin::new(10.0, 1),
                Bin::new(20.0, 3),
                Bin::new(30.0, 1)
            ])
            .mean(),
            20.0
        );
    }

    #[test]
    fn variance() {
        assert!(StreamHist::with_capacity(10).variance().is_nan());
        assert_eq!(
            StreamHist::from(vec![1.0, 2.0, 3.0, 4.0, 5.0]).variance(),
            2.0
        );
        // mean = (10 * 1 + 20 * 3 + 30 * 1) / 5 = (40 + 60) / 5 = 20
        // var = (1 * (10 - 20)^2 + 3 * (20 - 20)^2 + 1 * (30 - 20)^2) / 5 = (-10^2 + 10^2) / 5 = 200 / 5 = 40
        assert_eq!(
            StreamHist::from(vec![
                Bin::new(10.0, 1),
                Bin::new(20.0, 3),
                Bin::new(30.0, 1)
            ])
            .variance(),
            40.0
        );
    }
}

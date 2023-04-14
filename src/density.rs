use crate::hist::StreamHist;

/// Weighted [kernel density] estimator for the [`StreamHist`].
///
/// [kernel density]: https://en.wikipedia.org/wiki/Kernel_density_estimation
#[derive(Debug, Clone)]
pub struct KernelDensity {
    hist: StreamHist,
    /// Bandwidth of the kernels in the kernel density estimator. It is chosen automatically but may be adjusted.
    pub bandwidth: f64,
}

impl KernelDensity {
    /// Evaluate weighted kernel density estimator at the `value`.
    ///
    /// # Examples
    ///
    /// ```
    /// use streamhist::StreamHist;
    /// use streamhist::KernelDensity;
    ///
    /// let mut hist = StreamHist::from(vec![1.0, 0.5, 1.0, 3.5, 2.0, 3.0, 4.0, 2.5]);
    /// hist.resize(5);
    ///
    /// let kde = KernelDensity::from(hist);
    /// // the probability density is smaller for unseen vs seen values
    /// assert!(kde.density(0.0) < kde.density(3.5));
    /// ```
    pub fn density(&self, value: f64) -> f64 {
        if value.is_nan() {
            return f64::NAN;
        }
        self.hist.iter().fold(0.0, |acc, bin| {
            let u = (value - bin.mean) / self.bandwidth;
            let d = kernel::triangular(u) * bin.count as f64;
            acc + d
        }) / (self.hist.count() * self.bandwidth)
    }
}

impl From<StreamHist> for KernelDensity {
    fn from(hist: StreamHist) -> Self {
        let bandwidth = bandwidth::auto(&hist);
        KernelDensity { hist, bandwidth }
    }
}

mod kernel {
    #![allow(dead_code)]

    /// Triangular kernel `1 - |u|` for `value <= 1`.
    #[inline]
    pub fn triangular(value: f64) -> f64 {
        1.0 - value.abs().min(1.0)
    }

    /// Gaussian kernel `1/sqrt(2pi) * exp(-1/2 * u^2)`.
    #[inline]
    pub fn gaussian(value: f64) -> f64 {
        use std::f64::consts::PI;
        (-0.5 * value.powi(2)).exp() / (2.0 * PI).sqrt()
    }

    /// Epanechnikov kernel `3/4 * (1 - u^2)` for `value <= 1`.
    #[inline]
    pub fn epanechnikov(value: f64) -> f64 {
        let u = value.abs().min(1.0);
        0.75 * (1.0 - u.powi(2))
    }

    /// Uniform kernel `1/2` for `value <= 1`.
    #[inline]
    pub fn uniform(value: f64) -> f64 {
        if value.abs() <= 1.0 {
            0.5
        } else {
            0.0
        }
    }
}

pub mod bandwidth {
    #![allow(dead_code)]
    //! The rules of thumb for picking the bandwidth for kernel density estimators based on the
    //! [`StreamHist`] histograms.
    //!
    //! # Examples
    //!
    //! ```
    //! use streamhist::StreamHist;
    //! use streamhist::KernelDensity;
    //! use streamhist::bandwidth::bin_width;
    //!
    //! let mut hist = StreamHist::from(vec![1.0, 0.5, 1.0, 3.5, 2.0, 3.0, 4.0, 2.5]);
    //! hist.resize(5);
    //!
    //! let mut kde = KernelDensity::from(hist.clone());
    //! // pick the bandwidth based on the average bin width of the histogram
    //! kde.bandwidth = bin_width(&hist);
    //!
    //! // the probability density is smaller for unseen vs seen values
    //! assert!(kde.density(0.0) < kde.density(3.5));
    //! ```

    use crate::hist::StreamHist;

    /// Maximum of the `sturges` and `fd` bandwidth selection rules of thumb (as in Numpy).
    pub fn auto(hist: &StreamHist) -> f64 {
        sturges(hist).max(fd(hist))
    }

    /// Freedman's and Diaconis's bandwidth selection rule of thumb.
    pub fn fd(hist: &StreamHist) -> f64 {
        let n = hist.size as f64;
        2.0 * hist.iqr() * n.powf(-0.33)
    }

    /// Sturges's bandwidth selection rule of thumb.
    pub fn sturges(hist: &StreamHist) -> f64 {
        // k is the "optimal" number of bins, so the bandwidth is the average bin width (see bin_width below)
        let k = 1.0 + hist.count().log2();
        (hist.max - hist.min) / k
    }

    /// Use average bin width to select the bandwidth.
    pub fn bin_width(hist: &StreamHist) -> f64 {
        // as in the Sturges's selector but based on the actual number of bins
        (hist.max - hist.min) / hist.size as f64
    }

    /// Scott's bandwidth selection rule of thumb.
    pub fn scott(hist: &StreamHist) -> f64 {
        let n = hist.size as f64;
        3.5 * hist.stdev() * n.powf(-0.33)
    }

    /// Silverman's bandwidth selection rule of thumb.
    pub fn silverman(hist: &StreamHist) -> f64 {
        let n = hist.size as f64;
        let std = hist.stdev();
        let a = std.min(hist.iqr() / 1.34);
        0.9 * a * n.powf(-0.2)
    }

    impl StreamHist {
        /// Interquartile range calculated using the fast approximations for the quantiles.
        #[inline]
        fn iqr(&self) -> f64 {
            self.fast_quantile(0.75) - self.fast_quantile(0.25)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::KernelDensity;
    use crate::hist::StreamHist;

    #[test]
    fn empty_histogram() {
        let kde = KernelDensity::from(StreamHist::default());
        assert!(kde.density(0.0).is_nan());
    }

    #[test]
    fn density() {
        let mut hist = StreamHist::from(vec![1.0, 2.0, 2.0, 3.0, 4.0, 5.0]);
        hist.resize(3);
        let kde = KernelDensity::from(hist);

        assert!(kde.density(0.0) < kde.density(1.0));
        assert!(kde.density(6.0) < kde.density(5.0));
        assert!(kde.density(2.0) > kde.density(5.0));

        for x in 100..500 {
            let x = x as f64 / 100.0;
            assert!(kde.density(x) > 0.0);
        }
    }

    #[test]
    fn density_nan() {
        let hist = StreamHist::from(vec![1.0, 2.0, 2.0, 3.0, 4.0, 5.0]);
        let kde = KernelDensity::from(hist);
        assert!(kde.density(f64::NAN).is_nan());
    }
}

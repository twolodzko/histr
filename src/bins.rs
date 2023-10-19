use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::ops;

/// Bin of a [`StreamHist`](crate::hist::StreamHist) histogram.
///
/// The fields of `Bin` are private, it can be initialized using [`Bin::new`] or [`Bin::from<f64>`] functions.
/// Bins support the `+` operation for merging them.
#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize)]
pub struct Bin {
    /// Mean (value) of the bin. It needs to be a number (not `f64::NAN`, `f64::INFINITY`, or `f64::NEG_INFINITY`).
    pub(crate) mean: f64,
    /// The count of how many samples were aggregated to create the bin.
    pub(crate) count: u64,
}

impl Bin {
    /// Initialize new `Bin`.
    ///
    /// # Panics
    ///
    /// The `mean` needs to be a number. It will panic on `f64::NAN`, `f64::INFINITY`, or `f64::NEG_INFINITY`.
    ///
    /// # Examples
    ///
    /// ```
    /// use histr::Bin;
    ///
    /// let bin = &Bin::new(42.0, 2);
    /// let (mean, count): (f64, u64) = bin.into();
    /// assert_eq!(mean, 42.0);
    /// assert_eq!(count, 2);
    /// ```
    #[inline]
    pub fn new(mean: f64, count: u64) -> Self {
        assert!(!mean.is_nan() && mean.is_finite(), "{mean} is not a number");
        Bin { mean, count }
    }
}

impl From<f64> for Bin {
    /// Initialize a `Bin` from a value with the count equal to one.
    ///
    /// # Panics
    ///
    /// The `mean` needs to be a number. It will panic on `f64::NAN`, `f64::INFINITY`, or `f64::NEG_INFINITY`.
    ///
    /// # Examples
    ///
    /// ```
    /// use histr::Bin;
    ///
    /// let bin = &Bin::from(3.14);
    /// let (mean, count): (f64, u64) = bin.into();
    /// assert_eq!(mean, 3.14);
    /// assert_eq!(count, 1);
    /// ```
    fn from(mean: f64) -> Self {
        Bin::new(mean, 1)
    }
}

impl From<&Bin> for (f64, u64) {
    /// Convert `Bin` to a `(mean, count)` tuple.
    ///
    /// # Examples
    ///
    /// ```
    /// use histr::Bin;
    ///
    /// let bin = Bin::new(3.14, 5);
    /// assert_eq!(<(f64, u64)>::from(&bin), (3.14, 5));
    /// ```
    ///
    /// Use case example.
    ///
    /// ```
    /// use histr::Bin;
    /// use histr::StreamHist;
    ///
    /// let hist = StreamHist::from(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
    ///
    /// // extract means and counts of all the bins
    /// let (means, counts): (Vec<f64>, Vec<u64>) = hist.bins.iter().map(|bin| bin.into()).unzip();
    ///
    /// assert_eq!(means.len(), hist.bins.len());
    /// assert_eq!(counts.len(), hist.bins.len());
    /// ```
    fn from(bin: &Bin) -> Self {
        (bin.mean, bin.count)
    }
}

impl PartialEq for Bin {
    /// Compare the means of the bins.
    fn eq(&self, other: &Self) -> bool {
        self.mean == other.mean
    }
}

/// Compare the means of the bins.
///
/// # Examples
///
/// ```
/// use histr::Bin;
///
/// // the counts are ignored
/// assert!(Bin::new(3.14, 1) == Bin::new(3.14, 2));
/// ```
impl Eq for Bin {}

impl PartialOrd for Bin {
    #[allow(clippy::incorrect_partial_ord_impl_on_ord_type)]
    /// Compare the means of the bins.
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.mean.partial_cmp(&other.mean)
    }
}

impl Ord for Bin {
    /// Compare the means of the bins.
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}

impl ops::Add<Bin> for Bin {
    type Output = Bin;

    /// Merge two bins by taking their [weighted mean].
    ///
    /// After merging:
    /// * the `mean` of the new bin is the weighted mean of means of both bins weighted by the counts,
    /// * the `count` of the new bin is the sum of counts of both bins.
    ///
    /// See the [*A Streaming Parallel Decision Tree Algorithm* by Ben-Haim and Tom-Tov (2010)][paper] paper
    /// for more details.
    ///
    /// [paper]: https://jmlr.csail.mit.edu/papers/v11/ben-haim10a.html
    /// [weighted mean]: https://en.wikipedia.org/wiki/Weighted_arithmetic_mean
    ///
    /// # Examples
    ///
    /// ```
    /// use histr::Bin;
    ///
    /// // (1 * 2 + 2 * 3) / (2 + 3) = 8 / 5 = 1.6
    /// assert_eq!(Bin::new(1.0, 2) + Bin::new(2.0, 3), Bin::new(1.6, 5));
    /// ```
    fn add(self, rhs: Self) -> Self::Output {
        let total = self.count + rhs.count;
        let average = (self.mean * self.count as f64 + rhs.mean * rhs.count as f64) / total as f64;
        Bin::new(average, total)
    }
}

/// Sum the counts of all the bins.
#[inline]
pub(crate) fn sum_counts(bins: &[Bin]) -> u64 {
    bins.iter().fold(0, |acc, x| acc + x.count)
}

#[cfg(test)]
mod tests {
    use super::Bin;
    use test_case::test_case;

    #[test_case(f64::NAN ; "NaN")]
    #[test_case(f64::INFINITY ; "infinity")]
    #[test_case(f64::NEG_INFINITY ; "negative infinity")]
    #[should_panic]
    fn new_invalid(value: f64) {
        let _ = Bin::new(value, 1);
    }

    #[test_case(f64::NAN ; "NaN")]
    #[test_case(f64::INFINITY ; "infinity")]
    #[test_case(f64::NEG_INFINITY ; "negative infinity")]
    #[should_panic]
    fn from_invalid(value: f64) {
        let _ = Bin::from(value);
    }

    #[test]
    fn equal() {
        assert_eq!(Bin::new(0.0, 0), Bin::new(0.0, 0));
        assert_eq!(Bin::new(0.0, 0), Bin::new(0.0, 10));
        assert_eq!(Bin::new(10.0, 0), Bin::new(10.0, 10));
        assert_ne!(Bin::new(10.0, 0), Bin::new(0.0, 10));
    }

    #[test]
    fn compare() {
        assert!(Bin::new(0.0, 0) < Bin::new(1.0, 0));
        assert!(Bin::new(-1.0, 0) > Bin::new(-2.0, 0));
        assert!(Bin::new(0.0, 0) <= Bin::new(1.0, 0));
    }

    #[test]
    fn default() {
        assert_eq!(Bin::default(), Bin::new(0.0, 0))
    }
}

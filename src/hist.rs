use serde::{Deserialize, Serialize};

use crate::{
    bins::{sum_counts, Bin},
    is_sorted,
};
use std::vec::Vec;

/// Streaming histogram.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamHist {
    /// Sorted [`Bin`]s of the histogram.
    pub bins: Vec<Bin>,
    /// Smallest observed value.
    pub min: f64,
    /// Largest observed value.
    pub max: f64,
    /// Upper bound for the number of bins.
    pub size: usize,
}

impl StreamHist {
    /// Initialize an empty histogram with the number of bins equal to `size`.
    ///
    /// # Examples
    ///
    /// ```
    /// use histr::StreamHist;
    ///
    /// let mut hist = StreamHist::with_capacity(5);
    ///
    /// assert_eq!(hist.count(), 0.0); // has no data
    /// assert_eq!(hist.size, 5);      // but has the capacity
    ///
    /// for i in 1..=10 {
    ///     hist.insert(i as f64);
    /// }
    /// assert_eq!(hist.count(), 10.0); // it ingested ten values
    /// assert_eq!(hist.bins.len(), 5); // the number of bins is equal to size
    /// ```
    pub fn with_capacity(size: usize) -> Self {
        StreamHist {
            bins: Vec::with_capacity(size + 1),
            min: f64::NAN,
            max: f64::NAN,
            size,
        }
    }

    /// Adjust the number of bins in histogram.
    ///
    /// * If the number of bins in histogram is larger than the new `size`, the closest bins are merged.
    ///   The merging procedure is the same as used in [`StreamHist::insert`].
    /// * If the number of bins in histogram is smaller than the new `size`, the capacity of the histogram is
    ///   adjusted, so it can accommodate more bins in the future.
    ///
    /// # Examples
    ///
    /// ```
    /// use histr::StreamHist;
    ///
    /// let mut hist = StreamHist::from(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
    /// assert_eq!(hist.size, 5);
    /// assert_eq!(hist.bins.len(), 5);
    /// assert_eq!(hist.count(), 5.0);
    ///
    /// hist.resize(3);
    /// assert_eq!(hist.size, 3);       // changed
    /// assert_eq!(hist.bins.len(), 3); // changed
    /// assert_eq!(hist.count(), 5.0);
    /// ```
    pub fn resize(&mut self, size: usize) {
        self.size = size;
        self.trim()
    }

    /// Insert a new point to the histogram.
    ///
    /// The inserted `value` needs to be a number (not NaN or infinite), otherwise it panics.
    ///
    /// The "update" procedure that it uses is described by Ben-Haim and Tom-Tov (2010).
    ///
    /// # Panics
    ///
    /// The `value` needs to be a number. It will panic on `f64::NAN`, `f64::INFINITY`, or `f64::NEG_INFINITY`.
    ///
    /// # Examples
    ///
    /// ```
    /// use histr::StreamHist;
    ///
    /// let mut hist = StreamHist::with_capacity(5);
    /// hist.insert(1.0);
    /// hist.insert(2.0);
    ///
    /// let mut expected = StreamHist::from(vec![1.0, 2.0]);
    /// expected.resize(5);
    /// assert_eq!(hist, expected);
    /// ```
    pub fn insert(&mut self, value: f64) {
        if self.is_empty() {
            self.min = value;
            self.max = value;
            self.insert_at(0, value);
            return;
        }

        if value < self.min {
            self.min = value;
        }
        if value > self.max {
            self.max = value;
        }

        // Algorithm 1: Update Procedure from Ben-Haim & Tom-Tov (2010), p. 851
        let idx = self.partition_point(value);
        if idx < self.bins.len() && self.bins[idx].mean == value {
            self.increment_bin_count(idx);
        } else {
            self.insert_at(idx, value);
            self.trim();
        }

        debug_assert!(is_sorted(&self.bins));
    }

    /// Create a new bin with mean equal to `value` and insert it at the `index`.
    #[inline]
    fn insert_at(&mut self, index: usize, value: f64) {
        self.bins.insert(index, Bin::from(value));
    }

    /// Increment count of the bin at the `index`
    #[inline]
    fn increment_bin_count(&mut self, index: usize) {
        self.bins[index].count += 1;
    }

    /// Returns `true` if the histogram contains no data.
    ///
    /// # Examples
    ///
    /// ```
    /// use histr::StreamHist;
    ///
    /// let mut hist = StreamHist::with_capacity(10);
    /// assert!(hist.is_empty());
    ///
    /// hist.insert(3.14);
    /// assert!(!hist.is_empty());
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.bins.is_empty()
    }

    /// Find index such that all the bins before it are smaller or equal than the `value`.
    ///
    /// # Panics
    ///
    /// It panics when `value` is `f64::NAN`.
    #[inline]
    pub(crate) fn partition_point(&self, value: f64) -> usize {
        assert!(!value.is_nan(), "{value} is not a number");
        self.bins.partition_point(|x| x.mean < value)
    }

    /// Trim the histogram to have size not larger than `size`.
    fn trim(&mut self) {
        if self.size == 0 {
            self.bins = Vec::default();
        }
        while self.bins.len() > self.size {
            let idx = self.min_diff_index();
            self.merge_at(idx);
        }
        debug_assert!(is_sorted(&self.bins));
    }

    #[inline]
    fn merge_at(&mut self, idx: usize) {
        let updated = self.bins.remove(idx + 1) + self.bins[idx];
        self.bins[idx] = updated;
    }

    /// Find the index of the smallest difference of means between subsequent bins.
    fn min_diff_index(&self) -> usize {
        self.bins
            .windows(2)
            .map(|bins| bins[1].mean - bins[0].mean)
            .enumerate()
            .min_by(|(_, a), (_, b)| a.total_cmp(b))
            .map_or(0, |(index, _)| index)
    }

    /// The total count of all the values used to create the histogram.
    ///
    /// # Examples
    ///
    /// ```
    /// use histr::StreamHist;
    ///
    /// let mut hist = StreamHist::with_capacity(10);
    /// assert_eq!(hist.count(), 0.0);
    ///
    /// hist.insert(1.0);
    /// hist.insert(5.0);
    /// assert_eq!(hist.count(), 2.0);
    /// ```
    #[inline]
    pub fn count(&self) -> f64 {
        sum_counts(&self.bins) as f64
    }

    /// Merge two histograms.
    ///
    /// The `size` of the first histogram is preserved, while the `bins`, `min` and `max` are updated.
    /// Bins are updated by taking their weighted averages, the same as during the [`StreamHist::insert`] procedure.
    ///
    /// The "merge" procedure is described by Ben-Haim and Tom-Tov (2010).
    ///
    /// # Examples
    ///
    /// ```
    /// use histr::StreamHist;
    ///
    /// let mut hist1 = StreamHist::from(vec![1.0, 3.0, 5.0]);
    /// let hist2 = StreamHist::from(vec![2.0, 4.0, 6.0]);
    /// hist1.merge(hist2);
    ///
    /// let mut expected = StreamHist::from(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    /// expected.resize(3);
    /// assert_eq!(hist1, expected);
    /// ```
    pub fn merge(&mut self, other: Self) {
        // Algorithm 2: Merge Procedure from Ben-Haim & Tom-Tov (2010), p. 852
        self.bins.extend(other.bins);
        self.bins.sort();
        self.min = self.min.min(other.min);
        self.max = self.max.max(other.max);
        self.trim();
        debug_assert!(is_sorted(&self.bins));
    }

    /// Create an iterator over the bins.
    ///
    /// # Examples
    ///
    /// ```
    /// use histr::StreamHist;
    ///
    /// let mut hist = StreamHist::from(vec![2.0, 1.0, 1.0, 3.0]);
    /// hist.resize(3);
    /// let mut iter = hist.iter().map(|b| b.into());
    /// assert_eq!(iter.next(), Some((1.0, 2)));
    /// assert_eq!(iter.next(), Some((2.0, 1)));
    /// assert_eq!(iter.next(), Some((3.0, 1)));
    /// assert_eq!(iter.next(), None);
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = &Bin> {
        self.bins.iter()
    }
}

impl From<Vec<f64>> for StreamHist {
    /// Initialize histogram from a vector of values.
    ///
    /// # Panics
    ///
    /// All the `values` need to be a numbers. It will panic on any `f64::NAN`, `f64::INFINITY`, or `f64::NEG_INFINITY`.
    fn from(values: Vec<f64>) -> Self {
        if values.is_empty() {
            return StreamHist::default();
        }
        let mut bins: Vec<Bin> = values.iter().map(|x| Bin::from(*x)).collect();
        bins.sort();
        StreamHist {
            bins: bins.clone(),
            min: bins.first().unwrap().mean,
            max: bins.last().unwrap().mean,
            size: bins.len(),
        }
    }
}

impl From<Vec<Bin>> for StreamHist {
    fn from(bins: Vec<Bin>) -> Self {
        if bins.is_empty() {
            return StreamHist::default();
        }
        let mut bins = bins;
        bins.sort();
        StreamHist {
            bins: bins.clone(),
            min: bins.first().unwrap().mean,
            max: bins.last().unwrap().mean,
            size: bins.len(),
        }
    }
}

impl Default for StreamHist {
    /// Initialize empty histogram.
    ///
    /// # Examples
    ///
    /// ```
    /// use histr::StreamHist;
    ///
    /// let hist = StreamHist::default();
    /// assert!(hist.is_empty());
    /// assert_eq!(hist.size, 0);
    /// assert_eq!(hist.count(), 0.0);
    ///
    /// assert_eq!(StreamHist::default(), StreamHist::default());
    /// ```
    fn default() -> Self {
        StreamHist {
            bins: Vec::default(),
            min: f64::NAN,
            max: f64::NAN,
            size: 0,
        }
    }
}

impl PartialEq for StreamHist {
    fn eq(&self, other: &Self) -> bool {
        self.bins == other.bins
            && nan_or_eq(self.min, other.min)
            && nan_or_eq(self.max, other.max)
            && self.size == other.size
    }
}

/// Both values are either NaNs or are equal
#[inline]
fn nan_or_eq(a: f64, b: f64) -> bool {
    (a.is_nan() && b.is_nan()) || (a == b)
}

#[cfg(test)]
mod tests {
    use super::StreamHist;
    use crate::bins::Bin;
    use test_case::test_case;

    #[test]
    #[should_panic]
    fn partition_point_nan() {
        StreamHist::default().partition_point(f64::NAN);
    }

    #[test]
    fn partition_point() {
        let hist = StreamHist::with_capacity(3);
        assert_eq!(hist.partition_point(0.0), 0);
        assert_eq!(hist.partition_point(10.0), 0);

        let hist = StreamHist::from(vec![1.0, 2.0, 3.0]);
        assert_eq!(hist.partition_point(0.0), 0);
        assert_eq!(hist.partition_point(1.0), 0);
        assert_eq!(hist.partition_point(1.5), 1);
        assert_eq!(hist.partition_point(2.0), 1);
        assert_eq!(hist.partition_point(3.0), 2);
        assert_eq!(hist.partition_point(3.1), 3);
        assert_eq!(hist.partition_point(4.0), 3);

        assert_eq!(hist.partition_point(f64::NEG_INFINITY), 0);
        assert_eq!(hist.partition_point(f64::INFINITY), 3);
    }

    #[test_case(f64::NAN ; "NaN")]
    #[test_case(f64::INFINITY ; "infinity")]
    #[test_case(f64::NEG_INFINITY ; "negative infinity")]
    #[should_panic]
    fn insert_invalid(value: f64) {
        StreamHist::default().insert(value);
        StreamHist::from(vec![1.0, 2.0, 3.0]).insert(value);
    }

    #[test]
    fn insert() {
        let mut hist = StreamHist::with_capacity(3);

        // first element
        hist.insert(10.0);
        assert_eq!(
            hist,
            StreamHist {
                bins: vec![Bin::new(10.0, 2)],
                min: 10.0,
                max: 10.0,
                size: 3,
            }
        );
        // second and third elements
        hist.insert(30.0);
        hist.insert(20.0);
        assert_eq!(
            hist,
            StreamHist {
                bins: vec![Bin::from(10.0), Bin::from(20.0), Bin::from(30.0)],
                min: 10.0,
                max: 30.0,
                size: 3,
            }
        );
        // update count for the first element
        hist.insert(10.0);
        assert_eq!(
            hist,
            StreamHist {
                bins: vec![Bin::new(10.0, 2), Bin::from(20.0), Bin::from(30.0)],
                min: 10.0,
                max: 30.0,
                size: 3,
            }
        );

        // update count for the last element
        hist.insert(35.0);
        assert_eq!(
            hist,
            StreamHist {
                bins: vec![Bin::new(10.0, 2), Bin::from(20.0), Bin::new(32.5, 2)],
                min: 10.0,
                max: 35.0,
                size: 3,
            }
        );

        // update count for the first element
        hist.insert(1.0);
        assert_eq!(
            hist,
            StreamHist {
                bins: vec![Bin::new(7.0, 3), Bin::from(20.0), Bin::new(32.5, 2)],
                min: 1.0,
                max: 35.0,
                size: 3,
            }
        );

        // update count for the last element
        hist.insert(37.0);
        assert_eq!(
            hist,
            StreamHist {
                bins: vec![Bin::new(7.0, 3), Bin::from(20.0), Bin::new(34.0, 3)],
                min: 1.0,
                max: 37.0,
                size: 3,
            }
        );

        // update count for the second element
        hist.insert(22.0);
        assert_eq!(
            hist,
            StreamHist {
                bins: vec![Bin::new(7.0, 3), Bin::new(21.0, 2), Bin::new(34.0, 3)],
                min: 1.0,
                max: 37.0,
                size: 3,
            }
        );
    }

    #[test]
    fn merge_empty() {
        let mut hist = StreamHist::default();
        hist.merge(StreamHist::default());
        assert_eq!(hist, StreamHist::default());
    }

    #[test]
    fn merge() {
        let mut h1 = StreamHist::from(vec![1.0, 2.0, 3.0]);
        let h2 = StreamHist::from(vec![
            Bin::from(0.0),
            Bin::new(1.0, 2),
            Bin::from(2.5),
            Bin::new(6.0, 2),
        ]);
        h1.merge(h2);
        assert_eq!(
            h1,
            StreamHist {
                bins: vec![Bin::new(0.75, 4), Bin::new(2.5, 3), Bin::new(6.0, 2)],
                min: 0.0,
                max: 6.0,
                size: 3,
            }
        );
    }

    #[test]
    fn resize() {
        let mut hist = StreamHist::from(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0]);
        assert!(hist.size == 10);
        assert!(hist.bins.len() == 10);
        hist.resize(5);
        assert!(hist.size == 5);
        assert!(hist.bins.len() == 5);
        assert_eq!(
            hist,
            StreamHist {
                bins: vec![
                    Bin::new(1.5, 2),
                    Bin::new(3.5, 2),
                    Bin::new(5.5, 2),
                    Bin::new(7.5, 2),
                    Bin::new(9.5, 2),
                ],
                min: 1.0,
                max: 10.0,
                size: 5,
            }
        );

        hist.resize(20);
        assert!(hist.size == 20);
        assert_eq!(
            hist,
            StreamHist {
                bins: vec![
                    Bin::new(1.5, 2),
                    Bin::new(3.5, 2),
                    Bin::new(5.5, 2),
                    Bin::new(7.5, 2),
                    Bin::new(9.5, 2),
                ],
                min: 1.0,
                max: 10.0,
                size: 20,
            }
        );
    }

    #[test]
    fn is_empty() {
        assert!(StreamHist::default().is_empty());
        assert!(!StreamHist::from(vec![0.0]).is_empty());

        let mut hist = StreamHist::with_capacity(1);
        assert!(hist.is_empty());
        hist.insert(0.0);
        assert!(!hist.is_empty());

        hist.resize(0);
        assert!(hist.is_empty());
    }

    #[test]
    fn from_vec_is_sorted() {
        assert_eq!(
            StreamHist::from(vec![5.0, 1.0, 3.0, 4.0, 2.0]),
            StreamHist {
                bins: vec![
                    Bin::from(1.0),
                    Bin::from(2.0),
                    Bin::from(3.0),
                    Bin::from(4.0),
                    Bin::from(5.0)
                ],
                min: 1.0,
                max: 5.0,
                size: 5,
            }
        );
    }

    #[test_case(vec![f64::NAN] ; "NaN")]
    #[test_case(vec![f64::INFINITY] ; "infinity")]
    #[test_case(vec![f64::NEG_INFINITY] ; "negative infinity")]
    #[should_panic]
    fn from_vec_invalid(values: Vec<f64>) {
        let _ = StreamHist::from(values);
    }

    #[test]
    fn from_bins_is_sorted() {
        assert_eq!(
            StreamHist::from(vec![
                Bin::from(5.0),
                Bin::from(1.0),
                Bin::from(3.0),
                Bin::from(4.0),
                Bin::from(2.0)
            ]),
            StreamHist {
                bins: vec![
                    Bin::from(1.0),
                    Bin::from(2.0),
                    Bin::from(3.0),
                    Bin::from(4.0),
                    Bin::from(5.0)
                ],
                min: 1.0,
                max: 5.0,
                size: 5,
            }
        );
    }
}

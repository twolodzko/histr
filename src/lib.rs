//! Implementation of streaming histograms as described in the
//! [*A Streaming Parallel Decision Tree Algorithm* by Ben-Haim and Tom-Tov (2010)][paper].
//!
//! [paper]: https://jmlr.csail.mit.edu/papers/v11/ben-haim10a.html
//!
//! # Examples
//!
//! ```
//! use streamhist::StreamHist;
//! use streamhist::KernelDensity;
//!
//! // initialize a histogram with 10 bins
//! let mut hist = StreamHist::with_capacity(10);
//! // add some values to it
//! hist.insert(1.13);
//! hist.insert(2.67);
//! // ...
//!
//! // calculate statistics
//! println!("Mean = {}", hist.mean());
//!
//! // convert it to a kernel density estimator
//! let kde = KernelDensity::from(hist.clone());
//! println!("f({}) = {}", 3.14, kde.density(3.14));
//!
//! // print the histogram as a JSON
//! println!("{}", hist.to_json());
//! ```

mod bins;
mod density;
mod fast;
mod hist;
mod serde;
mod stats;

pub use self::bins::Bin;
pub use self::density::{bandwidth, KernelDensity};
pub use self::hist::StreamHist;

/// Check if a slice is sorted
fn is_sorted<T>(slice: &[T]) -> bool
where
    T: Ord,
{
    slice.windows(2).all(|w| w[0] <= w[1])
}

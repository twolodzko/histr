mod bin;
mod density;
mod fast;
mod hist;
mod serde;
mod stats;

pub use self::bin::Bin;
pub use self::density::{bandwidth, KernelDensity};
pub use self::hist::StreamHist;

/// Check if a slice is sorted
fn is_sorted<T>(slice: &[T]) -> bool
where
    T: Ord,
{
    slice.windows(2).all(|w| w[0] <= w[1])
}

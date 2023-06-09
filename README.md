# Streaming histograms

An implementation of the streaming histograms algorithm as described in
*[A Streaming Parallel Decision Tree Algorithm]* by Yael Ben-Haim and Elad Tom-Tov (2010).

The streaming histogram is defined in terms of bins

$$
(p_1, m_1), (p_2, m_2), \dots, (p_k, m_k)
$$

where $p_1 < p_2 < \dots < p_k$ are the means of the bins and $m_i$ are the counts of the values in the bin.
The sum of the counts is equal to the sample size used to create the histogram $\sum_i m_i = N$.

The histogram is created by treating the newly arriving datapoint $x$ as a new bin $(x, 1)$. The new bins are added
until reaching the pre-defined histogram size $k$. When the number of bins gets to $k+1$, the two bins with the
smallest difference between the means $p_{i+1} - p_i$ are merged by taking their weighted average

$$
\Big( \frac{p_i m_i + p_{i+1} m_{i+1}}{m_i + m_{i+1}},  m_i + m_{i+1}\Big)
$$

which is taken as a new bin replacing them. Histograms can also be resized or merged by merging the closest bins.

## Statistics

The [weighted mean] and [variance] of such bins can be used to approximate sample mean and variance. Yael Ben-Haim and
Elad Tom-Tov (2010) describe also algorithms for approximating the sample quantiles and empirical cumulative probability
distribution by applying the [trapezoidal rule] to interpolate between the bins.

## Kernel density estimation

Additionally, a weighted [kernel density estimator][kde] may be used for approximating the probability density
function of the data. The estimator is defined as

$$
\hat f(x) = \sum_{i=1}^k w_i K_h(x - p_i)
$$

with weights $w_i = \tfrac{m_i}{\sum_j m_j}$ and the [kernel] $K_h$ having the [bandwidth] $h$. Kernel densities
are closely related to histograms and there is a correspondence between the bandwidth of the kernel density estimator
and the width of the bins in the histogram.

## Command line interface

For example, the following command pipes the tab-separated file (ignoring the first line which is the header with
`tail -n +2`) to `histr`. The histogram is saved to a file (`-o hist.msgpack`) and printed. The saved histogram
could then be read again (with `-l hist.msgpack`) and be updated with new data.

```shell
$ tail -n +2 examples/old_faithful.tsv | histr -o hist.msgpack
mean    count
1.855946 56 ■■■■■■■■■■
2.162333 27 ■■■■■
2.436364 11 ■■
2.912500 4  ■
3.402125 8  ■
3.674462 13 ■■
3.987889 36 ■■■■■■
4.297208 48 ■■■■■■■■■
4.622364 55 ■■■■■■■■■■
4.919000 14 ■■■
```

Instead of piping, the file could be passed directly as `histr examples/old_faithful.tsv`, but then we would see
a warning printed to the standard error saying that parsing the first line (column name) failed.

It can be used with other command line programs, for example, to estimate the histogram of response times from ping.

```shell
$ ping google.com -c 20 | sed -n 's/.*time=\([0-9.]*\).*/\1/p' | histr -b 5
mean    count
8.965000 2  ■■
10.13000 10 ■■■■■■■■■■
11.20000 3  ■■■
13.22500 4  ■■■■
18.00000 1  ■
```

More details can be found in `histr -h` and some usage examples can be executed using the [Justfile] in this
repository with `just examples`.

## Library

Histr is also available as a Rust crate. It supports creating histograms from data or building them on-the-fly
in a streaming manner. The histograms can be resized and merged with other histograms. The crate exposes methods for
calculating the basic statistics (mean, standard deviation, median, quantiles) from the histograms and calculating
empirical cumulative distribution functions of kernel density estimators from them. 

```rust
use histr::StreamHist;
use histr::KernelDensity;

// initialize a histogram with 10 bins
let mut hist = StreamHist::with_capacity(10);
// add some values to it
hist.insert(1.13);
hist.insert(2.67);
// ...

// calculate statistics
println!("Mean = {}", hist.mean());

// convert it to a kernel density estimator
let kde = KernelDensity::from(hist.clone());
println!("f({}) = {}", 3.14, kde.density(3.14));

// print the histogram as a JSON
println!("{}", hist.to_json());
```

To use it, [specify it in `Cargo.toml`] as:

```toml
[dependencies]
histr = { git = "https://github.com/twolodzko/histr.git" }
```

## Other implementations

Similar implementations are also available in [carsonfarmer/streamhist] (Python), [maki-nage/distogram] (Python),
[VividCortex/gohistogram] (Go), [aaw/histosketch] (Go), [bigmlcom/histogram] (Java/Clojure), [aaw/histk] (C),
[malor/bhtt] (Rust), [jettify/streamhist] (Rust), etc. They vary in maturity and features, and some do not implement
the approach described by Yael Ben-Haim and Elad Tom-Tov (2010) or diverge from it.


 [A Streaming Parallel Decision Tree Algorithm]: https://jmlr.csail.mit.edu/papers/v11/ben-haim10a.html
 [carsonfarmer/streamhist]: https://github.com/carsonfarmer/streamhist
 [maki-nage/distogram]: https://github.com/maki-nage/distogram
 [malor/bhtt]: https://github.com/malor/bhtt
 [jettify/streamhist]: https://github.com/jettify/streamhist
 [aaw/histk]: https://github.com/aaw/histk
 [bigmlcom/histogram]: https://github.com/bigmlcom/histogram
 [VividCortex/gohistogram]: https://github.com/VividCortex/gohistogram
 [aaw/histosketch]: https://github.com/aaw/histosketch
 [weighted mean]: https://en.wikipedia.org/wiki/Weighted_arithmetic_mean
 [variance]: https://en.wikipedia.org/wiki/Weighted_arithmetic_mean#Weighted_sample_variance
 [trapezoidal rule]: https://en.wikipedia.org/wiki/Trapezoidal_rule
 [kde]: https://en.wikipedia.org/wiki/Kernel_density_estimation
 [kernel]: https://en.wikipedia.org/wiki/Kernel_(statistics)
 [bandwidth]: https://stats.stackexchange.com/a/226239/35989
 [Justfile]: https://github.com/casey/just
 [specify it in `Cargo.toml`]: https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html

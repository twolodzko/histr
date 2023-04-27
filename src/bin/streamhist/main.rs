#![cfg(feature = "build-binary")]
mod parse;

use crate::parse::parse;
use clap::error::ErrorKind;
use clap::{CommandFactory, Parser};
use float_pretty_print::PrettyPrintFloat;
use std::error::Error;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read};
use streamhist::{Bin, StreamHist};

const IO_ERROR_CODE: i32 = 74;

/// Streaming histogram
#[derive(Parser, Debug)]
struct Args {
    /// The number of bins
    #[arg(short = 'b', long, default_value_t = 10, value_name = "NUMBER")]
    number_of_bins: usize,

    /// Do a forced resize of the histogram to the number of bins given by the `-n` argument
    #[arg(short = 'r', long, default_value_t = false)]
    force_resize: bool,

    /// Initialize the histogram from the file (MessagePack unless the file extension is .json)
    #[arg(short, long, value_name = "PATH")]
    load_from: Option<String>,

    /// Save the histogram to a file at the given path (MessagePack unless the file extension is .json)
    #[arg(short, long, value_name = "PATH")]
    output_file: Option<String>,

    /// Use the nth field (column) of the input, where the fields are assumed to be separated with whitespaces
    #[arg(short, long, default_value_t = 1, value_name = "NUMBER")]
    field: usize,

    /// Print JSON of the histogram
    #[arg(short, long, default_value_t = false)]
    json: bool,

    /// Print the statistics
    #[arg(short, long, default_value_t = false)]
    statistics: bool,

    /// Don't print the summary of the histogram
    #[arg(short, long, default_value_t = false)]
    no_summary: bool,

    /// Maximal width of the histogram bars when displayed
    #[arg(short, long, default_value_t = 10, value_name = "NUMBER")]
    width: u32,

    /// Don't update the histogram (ignore FILE and stdin)
    #[arg(short, long, default_value_t = false)]
    ignore_input: bool,

    /// Input data file, if not given, the input is read from stdin
    file: Option<String>,
}

/// Initialize the histogram based on the provided arguments: fresh or from a file.
fn initialize_histogram(args: &Args) -> Result<StreamHist, Box<dyn Error>> {
    if let Some(ref from) = args.load_from {
        return read_histogram(from);
    }
    Ok(StreamHist::with_capacity(args.number_of_bins))
}

/// Read histogram from a file:
/// * when the file extension is .json (case-insensitive) as a JSON,
/// * otherwise treat it as a MessagePack file.
fn read_histogram(path: &str) -> Result<StreamHist, Box<dyn Error>> {
    let file = File::open(path).map_err(Box::new)?;
    if is_json(path) {
        StreamHist::read_json(file)
    } else {
        StreamHist::read_msgpack(file)
    }
}

/// Read the data from a file (if provided) or stdin and use it to update the histogram.
fn read_data(hist: &mut StreamHist, args: &Args) -> io::Result<()> {
    // A file or stdin
    let input: Box<dyn Read> = match &args.file {
        Some(path) => Box::new(File::open(path)?),
        None => Box::new(io::stdin()),
    };
    for (index, line) in BufReader::new(input).lines().enumerate() {
        match parse(line?, args.field - 1) {
            Ok(value) => hist.insert(value),
            // on parsing failure ignore this line and print warning to stderr
            Err(err) => eprintln!("line {}: {}", index + 1, err),
        }
    }
    Ok(())
}

/// Write the histogram to a file:
/// * when the file extension is .json (case-insensitive) as a JSON,
/// * otherwise as a MessagePack.
fn write(hist: &StreamHist, path: &str) -> Result<(), Box<dyn Error>> {
    let file = &mut File::create(path).map_err(Box::new)?;
    if is_json(path) {
        hist.write_json(file)
    } else {
        hist.write_msgpack(file)
    }
}

fn is_json(path: &str) -> bool {
    path.to_lowercase().ends_with(".json")
}

/// Print JSON for the histogram.
fn print_json(hist: &StreamHist) -> Result<(), Box<dyn Error>> {
    let stdout = &mut io::stdout().lock();
    hist.write_json(stdout)
}

/// Format the bin mean, count, and histogram bar as a string.
fn bin_to_string(bin: &Bin, max_count: u64, width: u32) -> String {
    let (mean, count) = bin.into();
    debug_assert!(count <= max_count);

    // the maximal width of the histogram bin is given by a command line option
    // it is scaled relatively to the maximum count of the bins
    let relative_count = count as f32 / max_count as f32;
    let bar_width = (relative_count * width as f32).round() as usize;
    debug_assert!(bar_width <= width as usize);
    let bar = &"■".repeat(bar_width);

    format!("{:8.3} {}\t{}", PrettyPrintFloat(mean), count, bar)
}

/// Print the histogram as text plot.
fn print_histogram(hist: &StreamHist, width: u32) {
    let max_count = hist.iter().fold(0, |acc, bin| {
        let (_, count) = bin.into();
        acc.max(count)
    });

    println!("mean\tcount");
    for bin in hist.iter() {
        let line = bin_to_string(bin, max_count, width);
        println!("{}", line);
    }
}

/// Print the summary statistics.
fn print_statistics(hist: &StreamHist) {
    for (name, value) in [
        ("Mean", hist.mean()),
        ("StDev", hist.stdev()),
        ("Min", hist.min),
        ("25% quantile", hist.quantile(0.25)),
        ("Median", hist.median()),
        ("75% quantile", hist.quantile(0.75)),
        ("Max", hist.max),
    ] {
        println!("{:14} {:<8.3}", name, PrettyPrintFloat(value));
    }
    println!("{:14} {:<8.0}", "Sample size", hist.count());
}

/// Parse and validate the CLI arguments
fn parse_args() -> Args {
    let args = Args::parse();
    if args.field < 1 {
        let mut cmd = Args::command();
        cmd.error(ErrorKind::InvalidValue, "field index needs to start at 1")
            .exit();
    }
    args
}

fn main() {
    let args = parse_args();

    let mut hist = initialize_histogram(&args)
        .map_err(|err| {
            eprintln!("failed to initialize the histogram: {}", err);
            std::process::exit(IO_ERROR_CODE);
        })
        .unwrap();

    if args.force_resize {
        hist.resize(args.number_of_bins);
    }

    if !&args.ignore_input {
        // Skip a histogram update regardless of the input
        if let Err(err) = read_data(&mut hist, &args) {
            eprintln!("failed to read the input: {}", err);
            std::process::exit(IO_ERROR_CODE);
        }
    }

    if args.json {
        if let Err(err) = print_json(&hist) {
            eprintln!("failed to print JSON: {}", err);
            std::process::exit(IO_ERROR_CODE);
        }
    }
    if !args.no_summary {
        print_histogram(&hist, args.width);
    }
    if args.statistics {
        print_statistics(&hist);
    }

    if let Some(path) = args.output_file {
        if let Err(err) = write(&hist, &path) {
            eprintln!("failed to write the output: {}", err);
            std::process::exit(IO_ERROR_CODE);
        }
    }
}

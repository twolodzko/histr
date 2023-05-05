
binary-file := "histr" + if os() == "windows" { ".exe" } else { "" }
flags := "--features build-binary"

# Run linter and all the tests
test: lint unit-test examples

# Run unit tests
unit-test:
	cargo test {{flags}}

# Run the linter
lint:
	cargo clippy {{flags}}

# Cleanup the build files
clean:
	rm -rf ./target/

# Build the Rust crate
lib:
	cargo build --release --lib

# Build the command line application
[unix]
binary:
	cargo build --release --bin histr {{flags}}
	cp ./target/release/{{binary-file}} .

# Install the package locally
install:
	cargo install --bin histr {{flags}} --path .

# Build the docs
docs:
	cargo doc --no-deps --open

# Count the number of code lines excluding the tests
lines:
	@ find . -type f -name "*.rs" -exec awk '1;/#[cfg\(test\)]/{exit}' {} \; | grep . | wc -l

# Run the examples
[unix]
examples: binary
	#!/usr/bin/env bash
	set -euo pipefail # fail fast
	set -v            # verbose
	tempdir=$(mktemp -d)

	# Print help
	./{{binary-file}} -h

	# Estimate simple histogram of the file sizes in the current directory
	ls -la | awk 'NR>1 {print $5}' | ./{{binary-file}} -b 5

	# The same as above, but instead of text plot show a JSON
	ls -la | awk 'NR>1 {print $5}' | ./{{binary-file}} -b 5 -j -n

	# Estimate histogram from tabular data and display it together with the summary statistics
	cat data/old_faithful.tsv | tail -n +2 | ./{{binary-file}} -s -b 15 -w 20 -f 1

	# As above, but log-transform the values using awk
	cat data/old_faithful.tsv | awk 'NR>1 {print log($1)}' | ./{{binary-file}} -s -b 15 -w 20

	# Save the histogram to a file
	cat data/old_faithful.tsv | tail -n +2 | ./{{binary-file}} -f 1 -b 15 -o $tempdir/hist.msgpack

	# Read and resize the saved histogram
	./{{binary-file}} -ir -b 10 -l $tempdir/hist.msgpack
	rm -rf $tempdir

[windows]
examples:
	@echo "Sorry, no examples for Windows for now."

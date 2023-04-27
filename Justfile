
binary := "streamhist"

# Run linter and all the tests
test: lint unit-test examples

# Run unit tests
unit-test:
    cargo test

# Run the linter
lint:
	cargo clippy

# Cleanup the build files
clean:
	rm -rf ./target/

# Build the binary
build: build-release
	cp target/release/streamhist ./{{binary}}

# Build the release binary
build-release:
	cargo build --release

# Build the optimized binary (smaller & faster)
build-optimized:
	cargo build --profile optimized

# Install the package locally
install:
	cargo install --path .

# Build the docs
docs:
	cargo doc --no-deps --open

# Count the number of code lines excluding the tests
lines:
	@ find . -type f -name "*.rs" -exec awk '1;/#[cfg\(test\)]/{exit}' {} \; | grep . | wc -l

# Run the examples
examples: build
	#!/usr/bin/env bash
	set -euo pipefail # fail fast
	set -v            # verbose
	tempdir=$(mktemp -d)

	# Print help
	./{{binary}} -h

	# Estimate simple histogram of the file sizes in the current directory
	ls -la | awk 'NR>1 {print $5}' | ./{{binary}} -b 5

	# The same as above, but instead of text plot show a JSON
	ls -la | awk 'NR>1 {print $5}' | ./{{binary}} -b 5 -j -n

	# Estimate histogram from tabular data and display it together with the summary statistics
	cat data/old_faithful.tsv | cut -f1 | tail -n +2 | ./{{binary}} -s -b 15 -w 20

	# As above, but log-transform the values using awk
	cat data/old_faithful.tsv | awk 'NR>1 {print log($1)}' | ./{{binary}} -s -b 15 -w 20

	# Save the histogram to a file
	cat data/old_faithful.tsv | tail -n +2 | ./{{binary}} -f 1 -b 15 -o $tempdir/hist.msgpack

	# Read and resize the saved histogram
	./{{binary}} -ir -b 10 -l $tempdir/hist.msgpack
	rm -rf $tempdir

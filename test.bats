#!/usr/bin/env bats

@test "By default use 10 bins" {
	run ./histr data/ping_data
	[ "$status" -eq 0 ]
	[ "${#lines[@]}" -eq 11 ]
}

@test "With -b 5 use 5 bins" {
	run ./histr -b 5 data/ping_data
	[ "$status" -eq 0 ]
	[ "${#lines[@]}" -eq 6 ]
}

@test "With -n don't print the histogram" {
	run ./histr -n data/ping_data
	[ "$status" -eq 0 ]
	[ "${#lines[@]}" -eq 0 ]
}

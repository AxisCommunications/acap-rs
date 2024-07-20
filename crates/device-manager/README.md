_Utilities for manipulating Axis devices_

Currently this crate focuses on restoring devices to a known, useful state.
It decomposes the problem into two parts:

- _restore_ any device in any state to a minimal baseline configuration.
- _initialize_ a restored device to a more useful baseline configuration.

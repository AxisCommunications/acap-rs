//! This library is not used anywhere.
//! Instead, it contains distilled patterns found in the FFI wrappers and tests that exercise them
//! such that the patterns can be run under miri.
#[cfg(test)]
mod sys;
#[cfg(test)]
mod tests;

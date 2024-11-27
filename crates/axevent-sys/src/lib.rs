pub mod mock_glib;
pub mod mock_glib_sys;

mod bindings;

// Always use the mocks for now because I could not immediately figure out how to set up the
// conditional compilation.
pub use bindings::*;

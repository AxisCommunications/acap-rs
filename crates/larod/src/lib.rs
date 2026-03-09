//! Safe Rust bindings for the [Larod (ML Inference) API](https://axiscommunications.github.io/acap-documentation/docs/api/src/api/larod/html/index.html).
//!
//! Larod provides access to hardware-accelerated ML inference on Axis cameras,
//! supporting various backends (CPU, GPU, DLPU/NPU) and model formats (TFLite,
//! ONNX, etc.).
//!
//! # Example
//!
//! ```no_run
//! use larod::Connection;
//!
//! let conn = Connection::new().expect("Failed to connect to larod");
//! let devices = conn.devices().expect("Failed to list devices");
//! for dev in &devices {
//!     println!("Device: {:?}", dev.name().unwrap());
//! }
//! ```
//!
//! # Typical workflow
//!
//! 1. Create a [`Connection`]
//! 2. Get a [`Device`] (e.g. `conn.device(c"cpu-tflite", 0)`)
//! 3. Load a [`Model`] from a file descriptor
//! 4. Allocate input/output [`Tensors`] via the connection
//! 5. Write input data to the input tensors' file descriptors
//! 6. Create a [`JobRequest`] and call [`JobRequest::run()`]
//! 7. Read results from the output tensors' file descriptors

#[macro_use]
mod error;
mod connection;
mod device;
mod job;
mod map;
mod model;
mod tensor;

pub use connection::Connection;
pub use device::Device;
pub use error::{Error, LarodError};
pub use job::JobRequest;
pub use map::Map;
pub use model::{Model, OwnedTensorPtrs};
pub use tensor::{TensorMut, TensorRef, Tensors};

// Re-export commonly used larod-sys types.
pub use larod_sys::{
    larodAccess, larodErrorCode, larodTensorDataType, larodTensorDims, larodTensorLayout,
    larodTensorPitches,
};

#[cfg(test)]
mod tests {
    use super::*;
    use expect_test::expect;

    #[test]
    fn error_display() {
        let err = LarodError::new_for_test(
            larodErrorCode::LAROD_ERROR_CONNECTION,
            "test error".to_string(),
        );
        expect![[r#"LAROD_ERROR_CONNECTION (-6): test error"#]].assert_eq(&err.to_string());
    }

    #[test]
    fn error_code_names() {
        let cases = [
            (larodErrorCode::LAROD_ERROR_NONE, "LAROD_ERROR_NONE"),
            (larodErrorCode::LAROD_ERROR_JOB, "LAROD_ERROR_JOB"),
            (
                larodErrorCode::LAROD_ERROR_LOAD_MODEL,
                "LAROD_ERROR_LOAD_MODEL",
            ),
            (larodErrorCode::LAROD_ERROR_FD, "LAROD_ERROR_FD"),
            (
                larodErrorCode::LAROD_ERROR_MODEL_NOT_FOUND,
                "LAROD_ERROR_MODEL_NOT_FOUND",
            ),
            (
                larodErrorCode::LAROD_ERROR_PERMISSION,
                "LAROD_ERROR_PERMISSION",
            ),
            (
                larodErrorCode::LAROD_ERROR_CONNECTION,
                "LAROD_ERROR_CONNECTION",
            ),
            (larodErrorCode(999), "LAROD_ERROR_UNKNOWN"),
        ];
        for (code, expected_name) in cases {
            let err = LarodError::new_for_test(code, String::new());
            assert_eq!(err.code_name(), expected_name, "for code {:?}", code);
        }
    }
}

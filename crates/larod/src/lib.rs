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

pub use connection::{Connection, ModelFuture};
pub use device::Device;
pub use error::{Error, LarodError};
pub use job::{JobCompletion, JobRequest};
pub use map::Map;
pub use model::{Model, OwnedTensorPtrs};
pub use tensor::{TensorMut, TensorRef, Tensors, TensorsIter, TensorsIterMut};

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
            (
                larodErrorCode::LAROD_ERROR_CREATE_SESSION,
                "LAROD_ERROR_CREATE_SESSION",
            ),
            (
                larodErrorCode::LAROD_ERROR_KILL_SESSION,
                "LAROD_ERROR_KILL_SESSION",
            ),
            (
                larodErrorCode::LAROD_ERROR_INVALID_CHIP_ID,
                "LAROD_ERROR_INVALID_CHIP_ID",
            ),
            (
                larodErrorCode::LAROD_ERROR_INVALID_ACCESS,
                "LAROD_ERROR_INVALID_ACCESS",
            ),
            (
                larodErrorCode::LAROD_ERROR_DELETE_MODEL,
                "LAROD_ERROR_DELETE_MODEL",
            ),
            (
                larodErrorCode::LAROD_ERROR_TENSOR_MISMATCH,
                "LAROD_ERROR_TENSOR_MISMATCH",
            ),
            (
                larodErrorCode::LAROD_ERROR_VERSION_MISMATCH,
                "LAROD_ERROR_VERSION_MISMATCH",
            ),
            (larodErrorCode::LAROD_ERROR_ALLOC, "LAROD_ERROR_ALLOC"),
            (
                larodErrorCode::LAROD_ERROR_POWER_NOT_AVAILABLE,
                "LAROD_ERROR_POWER_NOT_AVAILABLE",
            ),
            (larodErrorCode(999), "LAROD_ERROR_UNKNOWN"),
        ];
        for (code, expected_name) in cases {
            let err = LarodError::new_for_test(code, String::new());
            assert_eq!(err.code_name(), expected_name, "for code {:?}", code);
        }
    }
}

/// Tests that require the larod daemon and at least one inference device.
/// Run with: `cargo test --features device-tests` on Axis camera hardware.
#[cfg(feature = "device-tests")]
#[cfg(test)]
mod device_tests {
    use super::*;

    #[test]
    fn connect_and_list_sessions() {
        let conn = Connection::new().expect("connect");
        let sessions = conn.num_sessions().expect("num_sessions");
        assert!(sessions >= 1, "at least our own session should be counted");
    }

    #[test]
    fn list_devices() {
        let conn = Connection::new().expect("connect");
        let devices = conn.devices().expect("devices");
        assert!(!devices.is_empty(), "should have at least one device");

        for dev in &devices {
            let name = dev.name().expect("device name");
            assert!(!name.is_empty(), "device name should not be empty");
            let _instance = dev.instance().expect("device instance");
        }
    }

    #[test]
    fn get_device_by_name() {
        let conn = Connection::new().expect("connect");
        let devices = conn.devices().expect("devices");
        assert!(!devices.is_empty());

        // Look up the first device by its name and instance.
        let first = &devices[0];
        let name = first.name().expect("name");
        let instance = first.instance().expect("instance");

        let looked_up = conn.device(name, instance).expect("get device");
        assert_eq!(
            looked_up.name().expect("name"),
            name,
            "looked-up device name should match"
        );
    }

    #[test]
    fn map_round_trip() {
        let mut map = Map::new().expect("create map");

        map.set_str(c"key1", c"value1").expect("set_str");
        let v = map.get_str(c"key1").expect("get_str");
        assert_eq!(v.unwrap(), c"value1");

        map.set_int(c"num", 42).expect("set_int");
        assert_eq!(map.get_int(c"num").expect("get_int"), 42);

        map.set_int_arr2(c"pair", 10, 20).expect("set_int_arr2");
        assert_eq!(map.get_int_arr2(c"pair").expect("get_int_arr2"), [10, 20]);

        map.set_int_arr4(c"quad", 1, 2, 3, 4).expect("set_int_arr4");
        assert_eq!(map.get_int_arr4(c"quad").expect("get_int_arr4"), [1, 2, 3, 4]);
    }
}

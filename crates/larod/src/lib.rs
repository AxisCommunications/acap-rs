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
//! let conn = Connection::try_new().expect("Failed to connect to larod");
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
//!
//! # Compile-time safety checks
//!
//! Larod handles must not be sent to another thread because the underlying
//! library is not thread-safe.
//!
//! ```compile_fail
//! fn connections_are_not_send(conn: larod::Connection) {
//!     std::thread::scope(|scope| {
//!         scope.spawn(move || drop(conn));
//!     });
//! }
//! ```
//!
//! ```compile_fail
//! fn devices_are_not_send(device: larod::Device<'_>) {
//!     std::thread::scope(|scope| {
//!         scope.spawn(move || drop(device));
//!     });
//! }
//! ```
//!
//! ```compile_fail
//! fn models_are_not_send(model: larod::Model) {
//!     std::thread::scope(|scope| {
//!         scope.spawn(move || drop(model));
//!     });
//! }
//! ```
//!
//! ```compile_fail
//! fn maps_are_not_send(map: larod::Map) {
//!     std::thread::scope(|scope| {
//!         scope.spawn(move || drop(map));
//!     });
//! }
//! ```
//!
//! Connection-bound tensor arrays must not be sent to another thread while the
//! original connection can still be used.
//!
//! ```compile_fail
//! fn tensors_are_not_send(tensors: larod::Tensors<'_>) {
//!     std::thread::scope(|scope| {
//!         scope.spawn(move || drop(tensors));
//!     });
//! }
//! ```
//!
//! Connection-bound job requests must not be sent to another thread while the
//! original connection can still be used.
//!
//! ```compile_fail
//! fn job_requests_are_not_send(job: larod::JobRequest<'_>) {
//!     std::thread::scope(|scope| {
//!         scope.spawn(move || drop(job));
//!     });
//! }
//! ```
//!
//! Larod handle types must not be shared across threads either.
//!
//! ```compile_fail
//! fn assert_sync<T: Sync>() {}
//! assert_sync::<larod::Connection>();
//! ```
//!
//! ```compile_fail
//! fn assert_sync<T: Sync>() {}
//! assert_sync::<larod::Device<'static>>();
//! ```
//!
//! ```compile_fail
//! fn assert_sync<T: Sync>() {}
//! assert_sync::<larod::Model>();
//! ```
//!
//! ```compile_fail
//! fn assert_sync<T: Sync>() {}
//! assert_sync::<larod::Map>();
//! ```
//!
//! ```compile_fail
//! fn assert_sync<T: Sync>() {}
//! assert_sync::<larod::Tensors<'static>>();
//! ```
//!
//! ```compile_fail
//! fn assert_sync<T: Sync>() {}
//! assert_sync::<larod::JobRequest<'static>>();
//! ```
//!
//! ```compile_fail
//! fn assert_sync<T: Sync>() {}
//! assert_sync::<larod::TensorRef<'static>>();
//! ```
//!
//! ```compile_fail
//! fn assert_sync<T: Sync>() {}
//! assert_sync::<larod::TensorMut<'static>>();
//! ```
//!
//! ```compile_fail
//! fn assert_sync<T: Sync>() {}
//! assert_sync::<larod::TensorsIter<'static>>();
//! ```
//!
//! ```compile_fail
//! fn assert_sync<T: Sync>() {}
//! assert_sync::<larod::TensorsIterMut<'static>>();
//! ```
//!
//! Raw tensor file descriptor access requires an explicit unsafe block because
//! callers must uphold the fd lifetime and ownership contract.
//!
//! ```compile_fail
//! fn tensor_fd_getter_requires_unsafe(tensor: &larod::TensorRef<'_>) {
//!     let _ = tensor.fd();
//! }
//! ```
//!
//! ```compile_fail
//! fn tensor_fd_setter_requires_unsafe(tensor: &mut larod::TensorMut<'_>) {
//!     let _ = tensor.set_fd(0);
//! }
//! ```
//!
//! Safe tensor allocation only accepts validated file descriptor requirements,
//! not raw property flags.
//!
//! ```compile_fail
//! fn input_allocation_rejects_raw_flags(
//!     conn: &larod::Connection,
//!     model: &larod::Model,
//! ) {
//!     let _ = conn.alloc_model_inputs(model, 0_u32, None);
//! }
//! ```
//!
//! ```compile_fail
//! fn output_allocation_rejects_raw_flags(
//!     conn: &larod::Connection,
//!     model: &larod::Model,
//! ) {
//!     let _ = conn.alloc_model_outputs(model, 0_u32, None);
//! }
//! ```
//!
//! Raw allocation flags require an explicit unsafe block because callers must
//! uphold larod's flag requirements.
//!
//! ```compile_fail
//! fn input_raw_flags_require_unsafe(
//!     conn: &larod::Connection,
//!     model: &larod::Model,
//! ) {
//!     let _ = conn.alloc_model_inputs_with_flags(model, 0, None);
//! }
//! ```
//!
//! ```compile_fail
//! fn output_raw_flags_require_unsafe(
//!     conn: &larod::Connection,
//!     model: &larod::Model,
//! ) {
//!     let _ = conn.alloc_model_outputs_with_flags(model, 0, None);
//! }
//! ```
//!
//! Job request parameters are copied by larod and do not need to outlive the
//! call that sets them.
//!
//! ```no_run
//! fn job_params_can_be_short_lived(mut job: larod::JobRequest<'_>) {
//!     let map = larod::Map::try_new().unwrap();
//!     job.set_params(&map).unwrap();
//! }
//! ```

#[macro_use]
mod error;
mod connection;
mod device;
mod ffi;
mod job;
mod map;
mod model;
mod tensor;

pub use connection::Connection;
pub use device::Device;
pub use error::{Error, LarodError};
pub(crate) use ffi::with_larod;
pub use job::JobRequest;
// Re-export commonly used larod-sys types.
pub use larod_sys::{
    larodAccess, larodErrorCode, larodTensorDataType, larodTensorDims, larodTensorLayout,
    larodTensorPitches,
};
pub use map::Map;
pub use model::Model;
pub use tensor::{TensorMut, TensorRef, Tensors, TensorsIter, TensorsIterMut};

/// File descriptor capabilities required from buffers allocated by larod.
///
/// Requirements can be combined with `|`. Larod returns an error if the
/// model's device cannot allocate buffers satisfying the requested capabilities.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FdRequirements(u32);

impl FdRequirements {
    /// Let larod choose suitable capabilities based on the model.
    pub const AUTO: Self = Self(0);
    /// Require buffers that support `read` and `write` operations.
    pub const READ_WRITE: Self = Self(1 << 0);
    /// Require buffers that can be memory-mapped.
    pub const MAPPABLE: Self = Self(1 << 1);
    /// Require buffers backed by Linux DMA buffers.
    pub const DMA_BUF: Self = Self(1 << 2);
    /// Common requirements for disk-backed buffers.
    pub const DISK: Self = Self::READ_WRITE.union(Self::MAPPABLE);
    /// Common requirements for memory-mappable DMA buffers.
    pub const DMA: Self = Self::MAPPABLE.union(Self::DMA_BUF);

    /// Returns the corresponding `LAROD_FD_PROP_*` bitmask.
    pub const fn bits(self) -> u32 {
        self.0
    }

    const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl std::ops::BitOr for FdRequirements {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        self.union(rhs)
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::mpsc, time::Duration};

    use expect_test::expect;
    use larod_sys::larodErrorCode;

    use crate::LarodError;

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

    #[test]
    fn fd_requirements_match_larod_docs() {
        use crate::FdRequirements as Fd;

        assert_eq!(Fd::AUTO.bits(), 0);
        assert_eq!(Fd::READ_WRITE.bits(), 1);
        assert_eq!(Fd::MAPPABLE.bits(), 2);
        assert_eq!(Fd::DMA_BUF.bits(), 4);
        assert_eq!((Fd::READ_WRITE | Fd::MAPPABLE).bits(), 3);
        assert_eq!((Fd::READ_WRITE | Fd::DMA_BUF).bits(), 5);
        assert_eq!((Fd::MAPPABLE | Fd::DMA_BUF).bits(), 6);
        assert_eq!((Fd::READ_WRITE | Fd::MAPPABLE | Fd::DMA_BUF).bits(), 7);
        assert_eq!(Fd::DISK, Fd::READ_WRITE | Fd::MAPPABLE);
        assert_eq!(Fd::DMA, Fd::MAPPABLE | Fd::DMA_BUF);
    }

    #[test]
    fn larod_calls_are_serialized_across_threads() {
        let (first_entered_tx, first_entered_rx) = mpsc::channel();
        let (release_first_tx, release_first_rx) = mpsc::channel();
        let first = std::thread::spawn(move || {
            crate::with_larod(|| {
                first_entered_tx.send(()).unwrap();
                release_first_rx.recv().unwrap();
            });
        });
        first_entered_rx
            .recv_timeout(Duration::from_secs(1))
            .unwrap();

        let blocked = crate::ffi::try_with_larod(|| ()).is_none();

        release_first_tx.send(()).unwrap();
        first.join().unwrap();
        assert!(blocked);
        assert!(crate::ffi::try_with_larod(|| ()).is_some());
    }
}

/// Tests that require the larod daemon and at least one inference device.
/// Run with: `cargo test --features device-tests` on Axis camera hardware.
#[cfg(feature = "device-tests")]
#[cfg(test)]
mod device_tests {
    use crate::{Connection, Map};

    #[test]
    fn connect_and_list_sessions() {
        let conn = Connection::try_new().expect("connect");
        let sessions = conn.num_sessions().expect("num_sessions");
        assert!(sessions >= 1, "at least our own session should be counted");
    }

    #[test]
    fn list_devices() {
        let conn = Connection::try_new().expect("connect");
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
        let conn = Connection::try_new().expect("connect");
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
        let mut map = Map::try_new().expect("create map");

        map.set_str(c"key1", c"value1").expect("set_str");
        let v = map.get_str(c"key1").expect("get_str");
        assert_eq!(v.unwrap(), c"value1");

        map.set_int(c"num", 42).expect("set_int");
        assert_eq!(map.get_int(c"num").expect("get_int"), 42);

        map.set_int_arr2(c"pair", 10, 20).expect("set_int_arr2");
        assert_eq!(map.get_int_arr2(c"pair").expect("get_int_arr2"), [10, 20]);

        map.set_int_arr4(c"quad", 1, 2, 3, 4).expect("set_int_arr4");
        assert_eq!(
            map.get_int_arr4(c"quad").expect("get_int_arr4"),
            [1, 2, 3, 4]
        );
    }
}

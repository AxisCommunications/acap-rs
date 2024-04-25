//! An ergonomic API that is easy to use correctly
//!
//! It is meant to support all but the most exotic use cases in an idiomatic and intuitive way.
use std::ffi::{c_int, CStr};

use licensekey_sys::LicenseKeyState::*;
use Error::*;

/// An error indicating that the license key could not be verified.
#[derive(Clone, Copy, Debug, thiserror::Error)]
pub enum Error {
    #[error("validation error")]
    Validation,
    #[error("invalid version")]
    Version,
    #[error("expired date")]
    ExpiredDate,
    #[error("application id mismatch")]
    ApplicationIdMismatch,
    #[error("device id mismatch")]
    DeviceIdMismatch,
    #[error("missing fields")]
    MissingFields,
    #[error("invalid entry for application id")]
    InvalidApplicationId,
    #[error("invalid entry minimum major version")]
    InvalidMinMajor,
    #[error("invalid entry minimum minor version")]
    InvalidMinMinor,
    #[error("invalid entry maximum major version")]
    InvalidMaxMajor,
    #[error("invalid entry maximum minor version")]
    InvalidMaxMinor,
    #[error("key decoding fails")]
    KeyDecodingFails,
    #[error("invalid signature")]
    InvalidSignature,
}

/// Perform a license key check.
///
/// `app_name`, `app_id`, `major_version`, and `minor_version` should all match the corresponding
///  attribute in `manifest.json`.
pub fn verify(
    app_name: &CStr,
    app_id: c_int,
    major_version: c_int,
    minor_version: c_int,
) -> Result<(), Error> {
    let state = unsafe {
        licensekey_sys::licensekey_verify_ex(
            app_name.as_ptr(),
            app_id,
            major_version,
            minor_version,
            std::ptr::null(),
        )
    };

    debug_assert_eq!(NUM_LICENSEKEY_STATES as c_int, 14);
    match state {
        x if x == STATE_VALID as c_int => Ok(()),
        x if x == STATE_VALIDATION_ERROR as c_int => Err(Validation),
        x if x == STATE_INVALID_VERSION as c_int => Err(Version),
        x if x == STATE_EXPIRED_DATE as c_int => Err(ExpiredDate),
        x if x == STATE_ILLEGAL_APPLICATION_ID_MISMATCH as c_int => Err(ApplicationIdMismatch),
        x if x == STATE_ILLEGAL_DEVICE_ID_MISMATCH as c_int => Err(DeviceIdMismatch),
        x if x == STATE_ILLEGAL_MISSING_FIELDS as c_int => Err(MissingFields),
        x if x == STATE_ILLEGAL_INVALID_ENTRY_APPLICATION_ID as c_int => Err(InvalidApplicationId),
        x if x == STATE_ILLEGAL_INVALID_ENTRY_MIN_MAJOR as c_int => Err(InvalidMinMajor),
        x if x == STATE_ILLEGAL_INVALID_ENTRY_MIN_MINOR as c_int => Err(InvalidMinMinor),
        x if x == STATE_ILLEGAL_INVALID_ENTRY_MAX_MAJOR as c_int => Err(InvalidMaxMajor),
        x if x == STATE_ILLEGAL_INVALID_ENTRY_MAX_MINOR as c_int => Err(InvalidMaxMinor),
        x if x == STATE_ILLEGAL_KEY_DECODING_FAILS as c_int => Err(KeyDecodingFails),
        x if x == STATE_ILLEGAL_INVALID_SIGNATURE as c_int => Err(InvalidSignature),
        _ => panic!(
            "Expected a license key state less than {} but got {}",
            NUM_LICENSEKEY_STATES as c_int, state
        ),
    }
}

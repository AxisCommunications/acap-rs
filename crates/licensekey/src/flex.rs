//! A flexible API that closely follows the C API
//!
//! It is meant to support migrating users and power users by providing a safe API that
//! * has a similar structure to the C API, and
//! * allows everything that can be done (safely) with the C API.

use std::ffi::{c_int, CStr, CString};

use glib_sys::g_free;
use libc::free;

/// Perform a license key check.
///
/// # Arguments
///
/// * `app_name` - The name of the application. Must match APPNAME in package.conf.
/// * `app_id` - The application id of the application, assigned by Axis. Must match APPID in package.conf.
/// * `major_version` - The major version of the application. Must match APPMAJORVERSION in package.conf.
/// * `minor_version` - The minor version of the application. Must match APPMINORVERSION in package.conf.
///
/// # Returns
///
/// 1 on success, 0 on failure.
pub fn licensekey_verify(
    app_name: &CStr,
    app_id: c_int,
    major_version: c_int,
    minor_version: c_int,
) -> c_int {
    unsafe {
        licensekey_sys::licensekey_verify(app_name.as_ptr(), app_id, major_version, minor_version)
    }
}

/// Perform a license key check for ACAP3 applications.
///
/// # Arguments
///
/// * `app_name` - The name of the application. Must match APPNAME in package.conf.
/// * `app_id` - The application id of the application, assigned by Axis. Must match APPID in package.conf.
/// * `major_version` - The major version of the application. Must match APPMAJORVERSION in package.conf.
/// * `minor_version` - The minor version of the application. Must match APPMINORVERSION in package.conf.
/// * `licensekey_path` - Optional license key path meant to be used mainly for acap3 but is also available for acap2. Defaults to the acap2 license key folder.
///
/// # Returns
///
/// integer with license key state. The corresponding state message may be retrieved with function [`licensekey_get_state_string`].
pub fn licensekey_verify_ex(
    app_name: &CStr,
    app_id: c_int,
    major_version: c_int,
    minor_version: c_int,
    licensekey_path: Option<&CStr>,
) -> c_int {
    unsafe {
        licensekey_sys::licensekey_verify_ex(
            app_name.as_ref().as_ptr(),
            app_id,
            major_version,
            minor_version,
            match licensekey_path {
                None => std::ptr::null(),
                Some(p) => p.as_ptr(),
            },
        )
    }
}

/// Return the expiration date of the license.
///
/// # Arguments
///
/// * `app_name` - The name of the application.Must match APPNAME in package.conf.
/// * `licensekey_path` - Optional license key path meant to be used mainly for acap3 but is also available for acap2.
///   Defaults to the acap2 license key folder.
///
/// # Returns
///
/// * string with the expiration date in YYYY-MM-DD format.
/// * `None` if the expiration date couldn't be read.
pub fn licensekey_get_exp_date(app_name: &CStr, licensekey_path: Option<&CStr>) -> Option<CString> {
    unsafe {
        let ptr = licensekey_sys::licensekey_get_exp_date(
            app_name.as_ptr(),
            match licensekey_path {
                None => std::ptr::null(),
                Some(p) => p.as_ptr(),
            },
        );
        if ptr.is_null() {
            None
        } else {
            let retval = Some(CStr::from_ptr(ptr).to_owned());
            g_free(ptr as *mut _);
            retval
        }
    }
}

/// Return an explicatory message of a license key state for ACAP3 applications.
///
/// # Arguments
///
/// * `state_code` - Integer with license key state.
///
/// # Returns
///
/// * string with license key state message.
/// * `None` if state is not a valid error state.
pub fn licensekey_get_state_string(state_code: c_int) -> Option<CString> {
    unsafe {
        let ptr = licensekey_sys::licensekey_get_state_string(state_code as c_int);
        if ptr.is_null() {
            None
        } else {
            let value = CStr::from_ptr(ptr).into();
            free(ptr as *mut _);
            Some(value)
        }
    }
}

#[cfg(not(target_arch = "x86_64"))]
#[cfg(test)]
mod tests {
    use std::ffi::CString;

    use licensekey_sys::LicenseKeyState;

    use super::*;

    #[test]
    fn licensekey_verify_does_not_panic() {
        let app_name = CString::new("test_app").unwrap();
        _ = licensekey_verify(&app_name, 0, 1, 0);
    }

    #[test]
    fn licensekey_verify_ex_does_not_panic() {
        let app_name = CString::new("test_app").unwrap();
        _ = licensekey_verify_ex(&app_name, 0, 1, 0, None);
    }

    #[test]
    fn licensekey_get_exp_date_does_not_panic() {
        let app_name = CString::new("test_app").unwrap();
        _ = licensekey_get_exp_date(&app_name, None);
    }

    #[test]
    fn valid_license_key_states_have_a_unique_explanation() {
        // This seems like a reasonable property to ensure, and it implies some other desirable
        // properties including
        // * all variants can be converted to a string, and
        // * most strings are not empty.
        let mut explanations = std::collections::HashSet::new();
        for i in 0..LicenseKeyState::NUM_LICENSEKEY_STATES as c_int {
            let explanation = licensekey_get_state_string(i).unwrap();
            assert!(explanations.insert(explanation));
        }
    }
    #[test]
    fn invalid_license_key_states_have_no_explanation() {
        for i in [-1, LicenseKeyState::NUM_LICENSEKEY_STATES as c_int] {
            assert!(licensekey_get_state_string(i).is_none());
        }
    }
}

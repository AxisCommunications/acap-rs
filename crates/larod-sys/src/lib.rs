#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

#[cfg(not(target_arch = "x86_64"))]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(target_arch = "x86_64")]
include!("bindings.rs");

use std::ops::BitOr;

/// The error type returned from any fallible functions in this library.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("attempted conversion from an invalid access flag")]
    InvalidAccessFlag,
}

#[allow(non_camel_case_types)]
#[repr(u32)]
#[derive(Debug)]
pub enum FDAccessFlag {
    PROP_READWRITE = 1,
    PROP_MAP = 2,
    TYPE_DISK = 3,
    PROP_DMABUF = 4,
    TYPE_DMA = 6,
}

impl TryFrom<u32> for FDAccessFlag {
    type Error = Error;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(FDAccessFlag::PROP_READWRITE),
            2 => Ok(FDAccessFlag::PROP_MAP),
            3 => Ok(FDAccessFlag::TYPE_DISK),
            4 => Ok(FDAccessFlag::PROP_DMABUF),
            6 => Ok(FDAccessFlag::TYPE_DMA),
            _ => Err(Error::InvalidAccessFlag),
        }
    }
}

impl BitOr for FDAccessFlag {
    type Output = u32;

    // rhs is the "right-hand side" of the expression `a | b`
    fn bitor(self, rhs: Self) -> Self::Output {
        self as u32 | rhs as u32
    }
}

pub const LAROD_INVALID_MODEL_ID: u64 = u64::MAX;
pub const LAROD_INVALID_FD: i32 = i32::MIN;

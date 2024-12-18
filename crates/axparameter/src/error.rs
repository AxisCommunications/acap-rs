use axparameter_sys::{
    ax_parameter_error_quark, AXParameterErrorCode, AX_PARAMETER_DBUS_SETUP_ERROR,
    AX_PARAMETER_FILE_CREATE_ERROR, AX_PARAMETER_FILE_FD_ERROR, AX_PARAMETER_FILE_FORMAT_ERROR,
    AX_PARAMETER_FILE_LINK_ERROR, AX_PARAMETER_FILE_LOCK_ERROR, AX_PARAMETER_FILE_OPEN_ERROR,
    AX_PARAMETER_FILE_PATH_ERROR, AX_PARAMETER_FILE_RENAME_ERROR, AX_PARAMETER_FILE_UNLINK_ERROR,
    AX_PARAMETER_FILE_WRITE_ERROR, AX_PARAMETER_INVALID_ARG_ERROR, AX_PARAMETER_PARAM_ADDED_ERROR,
    AX_PARAMETER_PARAM_EXIST_ERROR, AX_PARAMETER_PARAM_GET_ERROR, AX_PARAMETER_PARAM_LIST_ERROR,
    AX_PARAMETER_PARAM_PATH_ERROR, AX_PARAMETER_PARAM_READ_GROUP_ERROR,
    AX_PARAMETER_PARAM_SET_ERROR, AX_PARAMETER_PARAM_SYNC_ERROR,
};
use glib::{
    error::ErrorDomain,
    translate::{from_glib, FromGlib, IntoGlib},
    Quark,
};

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Clone, Copy)]
#[non_exhaustive]
pub enum ParameterError {
    InvalidArg,
    FileFd,
    FileLock,
    FileOpen,
    FileFormat,
    FileCreate,
    FileWrite,
    FileLink,
    ParamList,
    ParamGet,
    ParamPath,
    ParamSync,
    ParamExist,
    ParamAdded,
    ParamReadGroup,
    ParamSet,
    DBusSetup,
    FileUnlink,
    FilePath,
    FileRename,
    __Unknown(i32),
}

impl IntoGlib for ParameterError {
    type GlibType = AXParameterErrorCode;

    #[inline]
    fn into_glib(self) -> AXParameterErrorCode {
        match self {
            Self::InvalidArg => AX_PARAMETER_INVALID_ARG_ERROR,
            Self::FileFd => AX_PARAMETER_FILE_FD_ERROR,
            Self::FileLock => AX_PARAMETER_FILE_LOCK_ERROR,
            Self::FileOpen => AX_PARAMETER_FILE_OPEN_ERROR,
            Self::FileFormat => AX_PARAMETER_FILE_FORMAT_ERROR,
            Self::FileCreate => AX_PARAMETER_FILE_CREATE_ERROR,
            Self::FileWrite => AX_PARAMETER_FILE_WRITE_ERROR,
            Self::FileLink => AX_PARAMETER_FILE_LINK_ERROR,
            Self::ParamList => AX_PARAMETER_PARAM_LIST_ERROR,
            Self::ParamGet => AX_PARAMETER_PARAM_GET_ERROR,
            Self::ParamPath => AX_PARAMETER_PARAM_PATH_ERROR,
            Self::ParamSync => AX_PARAMETER_PARAM_SYNC_ERROR,
            Self::ParamExist => AX_PARAMETER_PARAM_EXIST_ERROR,
            Self::ParamAdded => AX_PARAMETER_PARAM_ADDED_ERROR,
            Self::ParamReadGroup => AX_PARAMETER_PARAM_READ_GROUP_ERROR,
            Self::ParamSet => AX_PARAMETER_PARAM_SET_ERROR,
            Self::DBusSetup => AX_PARAMETER_DBUS_SETUP_ERROR,
            Self::FileUnlink => AX_PARAMETER_FILE_UNLINK_ERROR,
            Self::FilePath => AX_PARAMETER_FILE_PATH_ERROR,
            Self::FileRename => AX_PARAMETER_FILE_RENAME_ERROR,
            Self::__Unknown(value) => value,
        }
    }
}

impl FromGlib<AXParameterErrorCode> for ParameterError {
    #[inline]
    unsafe fn from_glib(value: AXParameterErrorCode) -> Self {
        match value {
            AX_PARAMETER_INVALID_ARG_ERROR => Self::InvalidArg,
            AX_PARAMETER_FILE_FD_ERROR => Self::FileFd,
            AX_PARAMETER_FILE_LOCK_ERROR => Self::FileLock,
            AX_PARAMETER_FILE_OPEN_ERROR => Self::FileOpen,
            AX_PARAMETER_FILE_FORMAT_ERROR => Self::FileFormat,
            AX_PARAMETER_FILE_CREATE_ERROR => Self::FileCreate,
            AX_PARAMETER_FILE_WRITE_ERROR => Self::FileWrite,
            AX_PARAMETER_FILE_LINK_ERROR => Self::FileLink,
            AX_PARAMETER_PARAM_LIST_ERROR => Self::ParamList,
            AX_PARAMETER_PARAM_GET_ERROR => Self::ParamGet,
            AX_PARAMETER_PARAM_PATH_ERROR => Self::ParamPath,
            AX_PARAMETER_PARAM_SYNC_ERROR => Self::ParamSync,
            AX_PARAMETER_PARAM_EXIST_ERROR => Self::ParamExist,
            AX_PARAMETER_PARAM_ADDED_ERROR => Self::ParamAdded,
            AX_PARAMETER_PARAM_READ_GROUP_ERROR => Self::ParamReadGroup,
            AX_PARAMETER_PARAM_SET_ERROR => Self::ParamSet,
            AX_PARAMETER_DBUS_SETUP_ERROR => Self::DBusSetup,
            AX_PARAMETER_FILE_UNLINK_ERROR => Self::FileUnlink,
            AX_PARAMETER_FILE_PATH_ERROR => Self::FilePath,
            AX_PARAMETER_FILE_RENAME_ERROR => Self::FileRename,
            value => Self::__Unknown(value),
        }
    }
}

impl ErrorDomain for ParameterError {
    #[inline]
    fn domain() -> Quark {
        unsafe { from_glib(ax_parameter_error_quark()) }
    }

    #[inline]
    fn code(self) -> i32 {
        self.into_glib()
    }

    #[inline]
    fn from(code: i32) -> Option<Self> {
        match unsafe { from_glib(code) } {
            Self::__Unknown(_) => None,
            value => Some(value),
        }
    }
}

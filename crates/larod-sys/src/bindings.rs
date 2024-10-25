/* automatically generated by rust-bindgen 0.69.4 */

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct larodDevice {
    _unused: [u8; 0],
}
pub const larodAccess_LAROD_ACCESS_INVALID: larodAccess = 0;
pub const larodAccess_LAROD_ACCESS_PRIVATE: larodAccess = 1;
pub const larodAccess_LAROD_ACCESS_PUBLIC: larodAccess = 2;
pub type larodAccess = ::std::os::raw::c_uint;
pub const larodErrorCode_LAROD_ERROR_NONE: larodErrorCode = 0;
pub const larodErrorCode_LAROD_ERROR_JOB: larodErrorCode = -1;
pub const larodErrorCode_LAROD_ERROR_LOAD_MODEL: larodErrorCode = -2;
pub const larodErrorCode_LAROD_ERROR_FD: larodErrorCode = -3;
pub const larodErrorCode_LAROD_ERROR_MODEL_NOT_FOUND: larodErrorCode = -4;
pub const larodErrorCode_LAROD_ERROR_PERMISSION: larodErrorCode = -5;
pub const larodErrorCode_LAROD_ERROR_CONNECTION: larodErrorCode = -6;
pub const larodErrorCode_LAROD_ERROR_CREATE_SESSION: larodErrorCode = -7;
pub const larodErrorCode_LAROD_ERROR_KILL_SESSION: larodErrorCode = -8;
pub const larodErrorCode_LAROD_ERROR_INVALID_CHIP_ID: larodErrorCode = -9;
pub const larodErrorCode_LAROD_ERROR_INVALID_ACCESS: larodErrorCode = -10;
pub const larodErrorCode_LAROD_ERROR_DELETE_MODEL: larodErrorCode = -11;
pub const larodErrorCode_LAROD_ERROR_TENSOR_MISMATCH: larodErrorCode = -12;
pub const larodErrorCode_LAROD_ERROR_VERSION_MISMATCH: larodErrorCode = -13;
pub const larodErrorCode_LAROD_ERROR_ALLOC: larodErrorCode = -14;
pub const larodErrorCode_LAROD_ERROR_POWER_NOT_AVAILABLE: larodErrorCode = -15;
pub const larodErrorCode_LAROD_ERROR_MAX_ERRNO: larodErrorCode = 1024;
pub type larodErrorCode = ::std::os::raw::c_int;
pub const larodTensorDataType_LAROD_TENSOR_DATA_TYPE_INVALID: larodTensorDataType = 0;
pub const larodTensorDataType_LAROD_TENSOR_DATA_TYPE_UNSPECIFIED: larodTensorDataType = 1;
pub const larodTensorDataType_LAROD_TENSOR_DATA_TYPE_BOOL: larodTensorDataType = 2;
pub const larodTensorDataType_LAROD_TENSOR_DATA_TYPE_UINT8: larodTensorDataType = 3;
pub const larodTensorDataType_LAROD_TENSOR_DATA_TYPE_INT8: larodTensorDataType = 4;
pub const larodTensorDataType_LAROD_TENSOR_DATA_TYPE_UINT16: larodTensorDataType = 5;
pub const larodTensorDataType_LAROD_TENSOR_DATA_TYPE_INT16: larodTensorDataType = 6;
pub const larodTensorDataType_LAROD_TENSOR_DATA_TYPE_UINT32: larodTensorDataType = 7;
pub const larodTensorDataType_LAROD_TENSOR_DATA_TYPE_INT32: larodTensorDataType = 8;
pub const larodTensorDataType_LAROD_TENSOR_DATA_TYPE_UINT64: larodTensorDataType = 9;
pub const larodTensorDataType_LAROD_TENSOR_DATA_TYPE_INT64: larodTensorDataType = 10;
pub const larodTensorDataType_LAROD_TENSOR_DATA_TYPE_FLOAT16: larodTensorDataType = 11;
pub const larodTensorDataType_LAROD_TENSOR_DATA_TYPE_FLOAT32: larodTensorDataType = 12;
pub const larodTensorDataType_LAROD_TENSOR_DATA_TYPE_FLOAT64: larodTensorDataType = 13;
pub const larodTensorDataType_LAROD_TENSOR_DATA_TYPE_MAX: larodTensorDataType = 14;
pub type larodTensorDataType = ::std::os::raw::c_uint;
pub const larodTensorLayout_LAROD_TENSOR_LAYOUT_INVALID: larodTensorLayout = 0;
pub const larodTensorLayout_LAROD_TENSOR_LAYOUT_UNSPECIFIED: larodTensorLayout = 1;
pub const larodTensorLayout_LAROD_TENSOR_LAYOUT_NHWC: larodTensorLayout = 2;
pub const larodTensorLayout_LAROD_TENSOR_LAYOUT_NCHW: larodTensorLayout = 3;
pub const larodTensorLayout_LAROD_TENSOR_LAYOUT_420SP: larodTensorLayout = 4;
pub const larodTensorLayout_LAROD_TENSOR_LAYOUT_MAX: larodTensorLayout = 5;
pub type larodTensorLayout = ::std::os::raw::c_uint;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct larodError {
    pub code: larodErrorCode,
    pub msg: *const ::std::os::raw::c_char,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct larodTensorDims {
    pub dims: [usize; 12usize],
    pub len: usize,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct larodTensorPitches {
    pub pitches: [usize; 12usize],
    pub len: usize,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct larodModel {
    _unused: [u8; 0],
}
pub type larodLoadModelCallback = ::std::option::Option<
    unsafe extern "C" fn(
        model: *mut larodModel,
        userData: *mut ::std::os::raw::c_void,
        error: *mut larodError,
    ),
>;
pub type larodRunJobCallback = ::std::option::Option<
    unsafe extern "C" fn(userData: *mut ::std::os::raw::c_void, error: *mut larodError),
>;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct larodConnection {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct larodJobRequest {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct larodTensor {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct larodMap {
    _unused: [u8; 0],
}
extern "C" {
    pub fn larodClearError(error: *mut *mut larodError);
}
extern "C" {
    pub fn larodConnect(conn: *mut *mut larodConnection, error: *mut *mut larodError) -> bool;
}
extern "C" {
    pub fn larodDisconnect(conn: *mut *mut larodConnection, error: *mut *mut larodError) -> bool;
}
extern "C" {
    pub fn larodGetNumSessions(
        conn: *mut larodConnection,
        numSessions: *mut u64,
        error: *mut *mut larodError,
    ) -> bool;
}
extern "C" {
    pub fn larodGetDevice(
        conn: *const larodConnection,
        name: *const ::std::os::raw::c_char,
        instance: u32,
        error: *mut *mut larodError,
    ) -> *const larodDevice;
}
extern "C" {
    pub fn larodGetDeviceName(
        dev: *const larodDevice,
        error: *mut *mut larodError,
    ) -> *const ::std::os::raw::c_char;
}
extern "C" {
    pub fn larodGetDeviceInstance(
        dev: *const larodDevice,
        instance: *mut u32,
        error: *mut *mut larodError,
    ) -> bool;
}
extern "C" {
    pub fn larodListDevices(
        conn: *mut larodConnection,
        numDevices: *mut usize,
        error: *mut *mut larodError,
    ) -> *mut *const larodDevice;
}
extern "C" {
    pub fn larodLoadModel(
        conn: *mut larodConnection,
        fd: ::std::os::raw::c_int,
        dev: *const larodDevice,
        access: larodAccess,
        name: *const ::std::os::raw::c_char,
        params: *const larodMap,
        error: *mut *mut larodError,
    ) -> *mut larodModel;
}
extern "C" {
    pub fn larodLoadModelAsync(
        conn: *mut larodConnection,
        inFd: ::std::os::raw::c_int,
        dev: *const larodDevice,
        access: larodAccess,
        name: *const ::std::os::raw::c_char,
        params: *const larodMap,
        callback: larodLoadModelCallback,
        userData: *mut ::std::os::raw::c_void,
        error: *mut *mut larodError,
    ) -> bool;
}
extern "C" {
    pub fn larodGetModel(
        conn: *mut larodConnection,
        modelId: u64,
        error: *mut *mut larodError,
    ) -> *mut larodModel;
}
extern "C" {
    pub fn larodGetModels(
        conn: *mut larodConnection,
        numModels: *mut usize,
        error: *mut *mut larodError,
    ) -> *mut *mut larodModel;
}
extern "C" {
    pub fn larodDestroyModel(model: *mut *mut larodModel);
}
extern "C" {
    pub fn larodDestroyModels(models: *mut *mut *mut larodModel, numModels: usize);
}
extern "C" {
    pub fn larodDeleteModel(
        conn: *mut larodConnection,
        model: *mut larodModel,
        error: *mut *mut larodError,
    ) -> bool;
}
extern "C" {
    pub fn larodGetModelId(model: *const larodModel, error: *mut *mut larodError) -> u64;
}
extern "C" {
    pub fn larodGetModelDevice(
        model: *const larodModel,
        error: *mut *mut larodError,
    ) -> *const larodDevice;
}
extern "C" {
    pub fn larodGetModelSize(model: *const larodModel, error: *mut *mut larodError) -> usize;
}
extern "C" {
    pub fn larodGetModelName(
        model: *const larodModel,
        error: *mut *mut larodError,
    ) -> *const ::std::os::raw::c_char;
}
extern "C" {
    pub fn larodGetModelAccess(
        model: *const larodModel,
        error: *mut *mut larodError,
    ) -> larodAccess;
}
extern "C" {
    pub fn larodGetModelNumInputs(model: *const larodModel, error: *mut *mut larodError) -> usize;
}
extern "C" {
    pub fn larodGetModelNumOutputs(model: *const larodModel, error: *mut *mut larodError) -> usize;
}
extern "C" {
    pub fn larodGetModelInputByteSizes(
        model: *const larodModel,
        numInputs: *mut usize,
        error: *mut *mut larodError,
    ) -> *mut usize;
}
extern "C" {
    pub fn larodGetModelOutputByteSizes(
        model: *const larodModel,
        numOutputs: *mut usize,
        error: *mut *mut larodError,
    ) -> *mut usize;
}
extern "C" {
    pub fn larodCreateModelInputs(
        model: *const larodModel,
        numTensors: *mut usize,
        error: *mut *mut larodError,
    ) -> *mut *mut larodTensor;
}
extern "C" {
    pub fn larodCreateModelOutputs(
        model: *const larodModel,
        numTensors: *mut usize,
        error: *mut *mut larodError,
    ) -> *mut *mut larodTensor;
}
extern "C" {
    pub fn larodAllocModelInputs(
        conn: *mut larodConnection,
        model: *const larodModel,
        fdPropFlags: u32,
        numTensors: *mut usize,
        params: *mut larodMap,
        error: *mut *mut larodError,
    ) -> *mut *mut larodTensor;
}
extern "C" {
    pub fn larodAllocModelOutputs(
        conn: *mut larodConnection,
        model: *const larodModel,
        fdPropFlags: u32,
        numTensors: *mut usize,
        params: *mut larodMap,
        error: *mut *mut larodError,
    ) -> *mut *mut larodTensor;
}
extern "C" {
    pub fn larodCreateTensors(
        numTensors: usize,
        error: *mut *mut larodError,
    ) -> *mut *mut larodTensor;
}
extern "C" {
    pub fn larodDestroyTensors(
        conn: *mut larodConnection,
        tensors: *mut *mut *mut larodTensor,
        numTensors: usize,
        error: *mut *mut larodError,
    ) -> bool;
}
extern "C" {
    pub fn larodSetTensorDims(
        tensor: *mut larodTensor,
        dims: *const larodTensorDims,
        error: *mut *mut larodError,
    ) -> bool;
}
extern "C" {
    pub fn larodGetTensorDims(
        tensor: *const larodTensor,
        error: *mut *mut larodError,
    ) -> *const larodTensorDims;
}
extern "C" {
    pub fn larodSetTensorPitches(
        tensor: *mut larodTensor,
        pitches: *const larodTensorPitches,
        error: *mut *mut larodError,
    ) -> bool;
}
extern "C" {
    pub fn larodGetTensorPitches(
        tensor: *const larodTensor,
        error: *mut *mut larodError,
    ) -> *const larodTensorPitches;
}
extern "C" {
    pub fn larodSetTensorDataType(
        tensor: *mut larodTensor,
        dataType: larodTensorDataType,
        error: *mut *mut larodError,
    ) -> bool;
}
extern "C" {
    pub fn larodGetTensorDataType(
        tensor: *const larodTensor,
        error: *mut *mut larodError,
    ) -> larodTensorDataType;
}
extern "C" {
    pub fn larodSetTensorLayout(
        tensor: *mut larodTensor,
        layout: larodTensorLayout,
        error: *mut *mut larodError,
    ) -> bool;
}
extern "C" {
    pub fn larodGetTensorLayout(
        tensor: *const larodTensor,
        error: *mut *mut larodError,
    ) -> larodTensorLayout;
}
extern "C" {
    pub fn larodSetTensorFd(
        tensor: *mut larodTensor,
        fd: ::std::os::raw::c_int,
        error: *mut *mut larodError,
    ) -> bool;
}
extern "C" {
    pub fn larodGetTensorFd(
        tensor: *const larodTensor,
        error: *mut *mut larodError,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn larodSetTensorFdSize(
        tensor: *mut larodTensor,
        size: usize,
        error: *mut *mut larodError,
    ) -> bool;
}
extern "C" {
    pub fn larodGetTensorFdSize(
        tensor: *const larodTensor,
        size: *mut usize,
        error: *mut *mut larodError,
    ) -> bool;
}
extern "C" {
    pub fn larodSetTensorFdOffset(
        tensor: *mut larodTensor,
        offset: i64,
        error: *mut *mut larodError,
    ) -> bool;
}
extern "C" {
    pub fn larodGetTensorFdOffset(tensor: *const larodTensor, error: *mut *mut larodError) -> i64;
}
extern "C" {
    pub fn larodTrackTensor(
        conn: *mut larodConnection,
        tensor: *mut larodTensor,
        error: *mut *mut larodError,
    ) -> bool;
}
extern "C" {
    pub fn larodSetTensorFdProps(
        tensor: *mut larodTensor,
        fdPropFlags: u32,
        error: *mut *mut larodError,
    ) -> bool;
}
extern "C" {
    pub fn larodGetTensorFdProps(
        tensor: *const larodTensor,
        fdPropFlags: *mut u32,
        error: *mut *mut larodError,
    ) -> bool;
}
extern "C" {
    pub fn larodGetTensorName(
        tensor: *const larodTensor,
        error: *mut *mut larodError,
    ) -> *const ::std::os::raw::c_char;
}
extern "C" {
    pub fn larodGetTensorByteSize(
        tensor: *const larodTensor,
        byteSize: *mut usize,
        error: *mut *mut larodError,
    ) -> bool;
}
extern "C" {
    pub fn larodCreateMap(error: *mut *mut larodError) -> *mut larodMap;
}
extern "C" {
    pub fn larodDestroyMap(map: *mut *mut larodMap);
}
extern "C" {
    pub fn larodMapSetStr(
        map: *mut larodMap,
        key: *const ::std::os::raw::c_char,
        value: *const ::std::os::raw::c_char,
        error: *mut *mut larodError,
    ) -> bool;
}
extern "C" {
    pub fn larodMapSetInt(
        map: *mut larodMap,
        key: *const ::std::os::raw::c_char,
        value: i64,
        error: *mut *mut larodError,
    ) -> bool;
}
extern "C" {
    pub fn larodMapSetIntArr2(
        map: *mut larodMap,
        key: *const ::std::os::raw::c_char,
        value0: i64,
        value1: i64,
        error: *mut *mut larodError,
    ) -> bool;
}
extern "C" {
    pub fn larodMapSetIntArr4(
        map: *mut larodMap,
        key: *const ::std::os::raw::c_char,
        value0: i64,
        value1: i64,
        value2: i64,
        value3: i64,
        error: *mut *mut larodError,
    ) -> bool;
}
extern "C" {
    pub fn larodMapGetStr(
        map: *mut larodMap,
        key: *const ::std::os::raw::c_char,
        error: *mut *mut larodError,
    ) -> *const ::std::os::raw::c_char;
}
extern "C" {
    pub fn larodMapGetInt(
        map: *mut larodMap,
        key: *const ::std::os::raw::c_char,
        value: *mut i64,
        error: *mut *mut larodError,
    ) -> bool;
}
extern "C" {
    pub fn larodMapGetIntArr2(
        map: *mut larodMap,
        key: *const ::std::os::raw::c_char,
        error: *mut *mut larodError,
    ) -> *const i64;
}
extern "C" {
    pub fn larodMapGetIntArr4(
        map: *mut larodMap,
        key: *const ::std::os::raw::c_char,
        error: *mut *mut larodError,
    ) -> *const i64;
}
extern "C" {
    pub fn larodCreateJobRequest(
        model: *const larodModel,
        inputTensors: *mut *mut larodTensor,
        numInputs: usize,
        outputTensors: *mut *mut larodTensor,
        numOutputs: usize,
        params: *mut larodMap,
        error: *mut *mut larodError,
    ) -> *mut larodJobRequest;
}
extern "C" {
    pub fn larodDestroyJobRequest(jobReq: *mut *mut larodJobRequest);
}
extern "C" {
    pub fn larodSetJobRequestModel(
        jobReq: *mut larodJobRequest,
        model: *const larodModel,
        error: *mut *mut larodError,
    ) -> bool;
}
extern "C" {
    pub fn larodSetJobRequestInputs(
        jobReq: *mut larodJobRequest,
        tensors: *mut *mut larodTensor,
        numTensors: usize,
        error: *mut *mut larodError,
    ) -> bool;
}
extern "C" {
    pub fn larodSetJobRequestOutputs(
        jobReq: *mut larodJobRequest,
        tensors: *mut *mut larodTensor,
        numTensors: usize,
        error: *mut *mut larodError,
    ) -> bool;
}
extern "C" {
    pub fn larodSetJobRequestPriority(
        jobReq: *mut larodJobRequest,
        priority: u8,
        error: *mut *mut larodError,
    ) -> bool;
}
extern "C" {
    pub fn larodSetJobRequestParams(
        jobReq: *mut larodJobRequest,
        params: *const larodMap,
        error: *mut *mut larodError,
    ) -> bool;
}
extern "C" {
    pub fn larodRunJob(
        conn: *mut larodConnection,
        jobReq: *const larodJobRequest,
        error: *mut *mut larodError,
    ) -> bool;
}
extern "C" {
    pub fn larodRunJobAsync(
        conn: *mut larodConnection,
        jobReq: *const larodJobRequest,
        callback: larodRunJobCallback,
        userData: *mut ::std::os::raw::c_void,
        error: *mut *mut larodError,
    ) -> bool;
}

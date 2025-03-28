/* automatically generated by rust-bindgen 0.69.5 */

pub const mdb_error_code_t_MDB_ERROR_NONE: mdb_error_code_t = 0;
pub const mdb_error_code_t_MDB_ERROR_ACCESS_DENIED: mdb_error_code_t = -1;
pub const mdb_error_code_t_MDB_ERROR_IO: mdb_error_code_t = -2;
pub const mdb_error_code_t_MDB_ERROR_MEMORY: mdb_error_code_t = -3;
pub const mdb_error_code_t_MDB_ERROR_OTHER: mdb_error_code_t = -4;
pub const mdb_error_code_t_MDB_ERROR_TIMEOUT: mdb_error_code_t = -5;
pub const mdb_error_code_t_MDB_ERROR_USER: mdb_error_code_t = -6;
pub const mdb_error_code_t_MDB_ERROR_VERSION_ERROR: mdb_error_code_t = -7;
pub const mdb_error_code_t_MDB_ERROR_TOO_MANY_PENDING_MESSAGES: mdb_error_code_t = -8;
pub type mdb_error_code_t = ::std::os::raw::c_int;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct mdb_error_t {
    pub code: mdb_error_code_t,
    pub message: *mut ::std::os::raw::c_char,
}
pub type mdb_on_error_t = ::std::option::Option<
    unsafe extern "C" fn(error: *const mdb_error_t, user_data: *mut ::std::os::raw::c_void),
>;
extern "C" {
    pub fn mdb_error_destroy(error: *mut *mut mdb_error_t);
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct mdb_connection {
    _unused: [u8; 0],
}
pub type mdb_connection_t = mdb_connection;
extern "C" {
    pub fn mdb_connection_create(
        on_error: mdb_on_error_t,
        user_data: *mut ::std::os::raw::c_void,
        error: *mut *mut mdb_error_t,
    ) -> *mut mdb_connection_t;
}
extern "C" {
    pub fn mdb_connection_destroy(self_: *mut *mut mdb_connection_t);
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct mdb_message {
    _unused: [u8; 0],
}
pub type mdb_message_t = mdb_message;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct mdb_message_payload {
    pub size: usize,
    pub data: *mut u8,
}
pub type mdb_message_payload_t = mdb_message_payload;
extern "C" {
    pub fn mdb_message_get_payload(message: *const mdb_message_t) -> *const mdb_message_payload_t;
}
extern "C" {
    pub fn mdb_message_get_timestamp(message: *const mdb_message_t) -> *const timespec;
}
extern "C" {
    pub fn mdb_message_create(
        timestamp: timespec,
        payload: *const u8,
        payload_size: usize,
        error: *mut *mut mdb_error_t,
    ) -> *mut mdb_message_t;
}
extern "C" {
    pub fn mdb_message_destroy(self_: *mut *mut mdb_message_t);
}
pub type mdb_on_done_t = ::std::option::Option<
    unsafe extern "C" fn(error: *const mdb_error_t, user_data: *mut ::std::os::raw::c_void),
>;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct mdb_dict {
    _unused: [u8; 0],
}
pub type mdb_dict_t = mdb_dict;
extern "C" {
    pub fn mdb_dict_set_str(
        self_: *mut mdb_dict_t,
        key: *const ::std::os::raw::c_char,
        value: *const ::std::os::raw::c_char,
        error: *mut *mut mdb_error_t,
    ) -> bool;
}
extern "C" {
    pub fn mdb_dict_get_str(
        self_: *const mdb_dict_t,
        key: *const ::std::os::raw::c_char,
        error: *mut *mut mdb_error_t,
    ) -> *const ::std::os::raw::c_char;
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct mdb_channel_info {
    _unused: [u8; 0],
}
pub type mdb_channel_info_t = mdb_channel_info;
extern "C" {
    pub fn mdb_channel_info_create(error: *mut *mut mdb_error_t) -> *mut mdb_channel_info_t;
}
extern "C" {
    pub fn mdb_channel_info_copy(
        self_: *const mdb_channel_info_t,
        error: *mut *mut mdb_error_t,
    ) -> *mut mdb_channel_info_t;
}
extern "C" {
    pub fn mdb_channel_info_get_application_data(
        self_: *const mdb_channel_info_t,
        error: *mut *mut mdb_error_t,
    ) -> *const mdb_dict_t;
}
extern "C" {
    pub fn mdb_channel_info_get_application_data_mutable(
        self_: *mut mdb_channel_info_t,
        error: *mut *mut mdb_error_t,
    ) -> *mut mdb_dict_t;
}
extern "C" {
    pub fn mdb_channel_info_destroy(self_: *mut *mut mdb_channel_info_t);
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct mdb_subscriber_config {
    _unused: [u8; 0],
}
pub type mdb_subscriber_config_t = mdb_subscriber_config;
pub type mdb_subscriber_on_message_t = ::std::option::Option<
    unsafe extern "C" fn(message: *const mdb_message_t, user_data: *mut ::std::os::raw::c_void),
>;
pub type mdb_subscriber_on_channel_registered_t = ::std::option::Option<
    unsafe extern "C" fn(info: *const mdb_channel_info_t, user_data: *mut ::std::os::raw::c_void),
>;
pub type mdb_subscriber_on_channel_unregistered_t =
    ::std::option::Option<unsafe extern "C" fn(user_data: *mut ::std::os::raw::c_void)>;
extern "C" {
    pub fn mdb_subscriber_config_create(
        topic: *const ::std::os::raw::c_char,
        source: *const ::std::os::raw::c_char,
        on_message: mdb_subscriber_on_message_t,
        user_data: *mut ::std::os::raw::c_void,
        error: *mut *mut mdb_error_t,
    ) -> *mut mdb_subscriber_config_t;
}
extern "C" {
    pub fn mdb_subscriber_config_disable_auto_subscribe(
        self_: *mut mdb_subscriber_config_t,
        error: *mut *mut mdb_error_t,
    ) -> bool;
}
extern "C" {
    pub fn mdb_subscriber_config_set_on_channel_registered_callback(
        self_: *mut mdb_subscriber_config_t,
        on_registered: mdb_subscriber_on_channel_registered_t,
        user_data: *mut ::std::os::raw::c_void,
        error: *mut *mut mdb_error_t,
    ) -> bool;
}
extern "C" {
    pub fn mdb_subscriber_config_set_on_channel_unregistered_callback(
        self_: *mut mdb_subscriber_config_t,
        on_unregistered: mdb_subscriber_on_channel_unregistered_t,
        user_data: *mut ::std::os::raw::c_void,
        error: *mut *mut mdb_error_t,
    ) -> bool;
}
extern "C" {
    pub fn mdb_subscriber_config_destroy(self_: *mut *mut mdb_subscriber_config_t);
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct mdb_subscriber {
    _unused: [u8; 0],
}
pub type mdb_subscriber_t = mdb_subscriber;
extern "C" {
    pub fn mdb_subscriber_create_async(
        connection: *mut mdb_connection_t,
        config: *mut mdb_subscriber_config_t,
        on_done: mdb_on_done_t,
        user_data: *mut ::std::os::raw::c_void,
        error: *mut *mut mdb_error_t,
    ) -> *mut mdb_subscriber_t;
}
extern "C" {
    pub fn mdb_subscriber_manual_subscribe_async(
        self_: *mut mdb_subscriber_t,
        on_done: mdb_on_done_t,
        user_data: *mut ::std::os::raw::c_void,
        error: *mut *mut mdb_error_t,
    ) -> bool;
}
extern "C" {
    pub fn mdb_subscriber_destroy(self_: *mut *mut mdb_subscriber_t);
}

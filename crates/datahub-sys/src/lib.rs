#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(improper_ctypes)]

#[cfg(not(target_arch = "x86_64"))]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(target_arch = "x86_64")]
include!("./bindings.rs");

// The C headers define the constants below as object-like macros with casts, e.g.
// `#define DH_ERR_INVALID_PARAMS ((DHErrorCode)0)`, which bindgen cannot evaluate,
// so they are transcribed manually.

pub const DH_ERR_INVALID_PARAMS: DHErrorCode = 0;
pub const DH_ERROR_DATA_TOO_BIG: DHErrorCode = 1;
pub const DH_ERR_NOT_CONNECTED: DHErrorCode = 2;
pub const DH_ERR_ALREADY_CONNECTED: DHErrorCode = 3;
pub const DH_ERR_UNKNOWN_ERROR: DHErrorCode = 4;
pub const DH_ERR_INTERNAL_ERROR: DHErrorCode = 5;
pub const DH_ERR_INVALID_ID: DHErrorCode = 6;
pub const DH_ERR_INVALID_TOPIC: DHErrorCode = 7;
pub const DH_ERR_NOT_INITIALIZED: DHErrorCode = 8;
pub const DH_ERR_MAX_CLIENTS: DHErrorCode = 9;
pub const DH_ERR_MAX_CONNECTIONS: DHErrorCode = 10;
pub const DH_ERR_AUTHENTICATION_FAILED: DHErrorCode = 11;
pub const DH_ERR_TOPIC_EXISTS: DHErrorCode = 12;
pub const DH_ERR_MAX_TOPIC: DHErrorCode = 13;
pub const DH_ERR_CONNECTION_ERROR: DHErrorCode = 14;
pub const DH_ERR_INVALID_DATA: DHErrorCode = 15;
pub const DH_ERR_INVALID_INSTANCE: DHErrorCode = 16;
pub const DH_ERR_INVALID_KEYS: DHErrorCode = 17;
pub const DH_ERR_INVALID_SUBSCRIPTION: DHErrorCode = 18;
pub const DH_ERR_INSTANCE_EXISTS: DHErrorCode = 19;
pub const DH_ERR_INSTANCES_NOT_SUPPORTED: DHErrorCode = 20;
pub const DH_ERR_MAX_INSTANCES: DHErrorCode = 21;
pub const DH_ERR_MAX_SUBSCRIPTIONS: DHErrorCode = 22;
pub const DH_ERR_AUTHORIZATION_FAILED: DHErrorCode = 23;
pub const DH_ERR_MAX_PRODUCTIONS: DHErrorCode = 24;
pub const DH_ERR_INVALID_PRODUCTION: DHErrorCode = 25;
pub const DH_ERR_INVALID_REQUEST: DHErrorCode = 26;
pub const DH_ERR_CREDENTIAL_ERROR: DHErrorCode = 27;

pub const DH_CONN_DISCONNECTED: DHConnectionState = 0;
pub const DH_CONN_CONNECTED: DHConnectionState = 1;

pub const DH_LOG_OFF: DHLogLevel = 0;
pub const DH_LOG_CRITICAL: DHLogLevel = 1;
pub const DH_LOG_ERROR: DHLogLevel = 2;
pub const DH_LOG_WARNING: DHLogLevel = 3;
pub const DH_LOG_INFO: DHLogLevel = 4;
pub const DH_LOG_DEBUG: DHLogLevel = 5;
pub const DH_LOG_TRACE: DHLogLevel = 6;

pub const DH_LOG_TARGET_CONSOLE: DHLogTarget = 0;
pub const DH_LOG_TARGET_SYSLOG: DHLogTarget = 1;

pub const DH_TOPIC_CREATED: DHTopicUpdateType = 0;
pub const DH_TOPIC_DELETED: DHTopicUpdateType = 1;

pub const DH_TOPIC_INSTANCE_CREATED: DHTopicInstanceUpdateType = 0;
pub const DH_TOPIC_INSTANCE_DELETED: DHTopicInstanceUpdateType = 1;

pub const DH_START_FROM_NOW: DHStartFrom = 0;
pub const DH_START_FROM_OLDEST: DHStartFrom = 1;

pub const DH_CONSUMER_NO_MATCH: DHConsumerMatchStatus = 0;
pub const DH_CONSUMER_MATCH: DHConsumerMatchStatus = 1;

use std::{
    ffi::{c_char, CStr},
    marker::PhantomData,
};

use axparameter_sys::{
    ax_parameter_add, ax_parameter_free, ax_parameter_get, ax_parameter_list, ax_parameter_new,
    ax_parameter_register_callback, ax_parameter_remove, ax_parameter_set,
    ax_parameter_unregister_callback, AXParameter,
};
use glib::translate::{from_glib, from_glib_full, from_glib_none, IntoGlib, ToGlibPtr};
use glib_sys::gpointer;

use super::types::{ControlWord, ParameterValue};

unsafe extern "C" fn trampoline_ax_parameter<F: Fn(&str, &str) + Send + Sync + 'static>(
    name: *const c_char,
    value: *const c_char,
    func: gpointer,
) {
    let func: &F = &*(func as *const F);
    let name_c = CStr::from_ptr(name);
    let value_c = CStr::from_ptr(value);

    func(name_c.to_str().unwrap(), value_c.to_str().unwrap());
}

#[derive(Debug)]
pub struct Parameter {
    ptr: *mut AXParameter,
    _phantom: PhantomData<AXParameter>,
}

unsafe impl Send for Parameter {}
unsafe impl Sync for Parameter {}

impl Parameter {
    pub fn new(name: &str) -> Result<Self, glib::error::Error> {
        unsafe {
            let mut error = std::ptr::null_mut();
            let ret = ax_parameter_new(name.to_glib_none().0, &mut error);

            if error.is_null() {
                Ok(Self {
                    ptr: ret,
                    _phantom: PhantomData,
                })
            } else {
                Err(from_glib_full(error))
            }
        }
    }

    pub fn add<T: ParameterValue>(
        &self,
        name: &str,
        control_word: Option<ControlWord>,
        initial_value: T,
    ) -> Result<(), glib::error::Error> {
        let mut r#type = T::to_param_type();
        match control_word {
            None => (),
            Some(ControlWord::Hidden) => r#type.insert_str(0, "hidden:"),
            Some(ControlWord::NoSync) => r#type.insert_str(0, "nosync:"),
            Some(ControlWord::ReadOnly) => r#type.insert_str(0, "readonly:"),
        }

        let initial_value_str = initial_value.to_param_string();
        unsafe {
            let mut error = std::ptr::null_mut();
            let _: bool = from_glib(ax_parameter_add(
                self.ptr,
                name.to_glib_none().0,
                initial_value_str.to_glib_none().0,
                r#type.to_glib_none().0,
                &mut error,
            ));

            if error.is_null() {
                Ok(())
            } else {
                Err(from_glib_full(error))
            }
        }
    }

    pub fn remove(&self, name: &str) -> Result<(), glib::error::Error> {
        unsafe {
            let mut error = std::ptr::null_mut();
            let _: bool = from_glib(ax_parameter_remove(
                self.ptr,
                name.to_glib_none().0,
                &mut error,
            ));

            if error.is_null() {
                Ok(())
            } else {
                Err(from_glib_full(error))
            }
        }
    }

    pub fn set<T: ParameterValue>(
        &self,
        name: &str,
        value: T,
        do_sync: bool,
    ) -> Result<(), glib::error::Error> {
        let value_str = value.to_param_string();
        unsafe {
            let mut error = std::ptr::null_mut();
            let _: bool = from_glib(ax_parameter_set(
                self.ptr,
                name.to_glib_none().0,
                value_str.to_glib_none().0,
                do_sync.into_glib(),
                &mut error,
            ));

            if error.is_null() {
                Ok(())
            } else {
                Err(from_glib_full(error))
            }
        }
    }

    pub fn get<T: ParameterValue>(&self, name: &str) -> Result<T, glib::error::Error> {
        unsafe {
            let mut value_ptr = std::ptr::null_mut();
            let mut error = std::ptr::null_mut();
            let _: bool = from_glib(ax_parameter_get(
                self.ptr,
                name.to_glib_none().0,
                &mut value_ptr,
                &mut error,
            ));

            if error.is_null() {
                let value_str: String = from_glib_full(value_ptr);
                T::from_param_string(value_str)
            } else {
                Err(from_glib_full(error))
            }
        }
    }

    pub fn list(&self) -> Result<Vec<String>, glib::error::Error> {
        unsafe {
            let mut error = std::ptr::null_mut();
            let list = ax_parameter_list(self.ptr, &mut error);

            if error.is_null() {
                let mut vec: Vec<String> = Vec::new();
                let mut list_tmp = glib_sys::g_list_first(list);
                while !list_tmp.is_null() {
                    vec.push(from_glib_none((*list_tmp).data as *mut c_char));
                    glib_sys::g_free((*list_tmp).data);
                    list_tmp = (*list_tmp).next;
                }
                glib_sys::g_list_free(list);
                Ok(vec)
            } else {
                Err(from_glib_full(error))
            }
        }
    }

    pub fn register_callback<F>(&self, name: &str, callback: F) -> Result<(), glib::error::Error>
    where
        F: Fn(&str, &str) + Send + Sync + 'static,
    {
        unsafe {
            let func: Box<F> = Box::new(callback);
            let mut error = std::ptr::null_mut();
            let _: bool = from_glib(ax_parameter_register_callback(
                self.ptr,
                name.to_glib_none().0,
                Some(trampoline_ax_parameter::<F>),
                Box::into_raw(func) as gpointer,
                &mut error,
            ));

            if error.is_null() {
                Ok(())
            } else {
                Err(from_glib_full(error))
            }
        }
    }

    pub fn unregister_callback(&self, name: &str) {
        unsafe {
            ax_parameter_unregister_callback(self.ptr, name.to_glib_none().0);
        }
    }
}

impl Drop for Parameter {
    fn drop(&mut self) {
        unsafe {
            ax_parameter_free(self.ptr);
        }
    }
}

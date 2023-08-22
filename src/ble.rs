use core::ffi;
use cty;

extern "C" {
    fn lovense_is_connected() -> cty::uint8_t;
    fn lovense_get_name() -> *const cty::c_char;
}

pub fn ble_is_connected() -> bool {
    let res = unsafe { lovense_is_connected() };
    return res > 0;
}

pub fn ble_get_name() -> &'static str {
    let c_buf = unsafe { lovense_get_name() };
    let c_str: &ffi::CStr = unsafe { ffi::CStr::from_ptr(c_buf) };
    c_str.to_str().unwrap()
}

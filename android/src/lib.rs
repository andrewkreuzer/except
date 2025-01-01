use std::ffi::{CStr, CString};
use std::os::raw::c_char;

#[unsafe(no_mangle)]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn rust_greeting(to: *const c_char) -> *mut c_char {
    let c_str = unsafe { CStr::from_ptr(to) };
    let recipient = match c_str.to_str() {
        Err(_) => "there",
        Ok(string) => string,
    };

    CString::new("Hello ".to_owned() + recipient)
        .unwrap()
        .into_raw()
}

#[allow(non_snake_case)]
pub mod android {
    extern crate jni;

    use self::jni::JNIEnv;
    use self::jni::objects::{JClass, JString};
    use self::jni::sys::jstring;
    use super::*;

    #[unsafe(no_mangle)]
    #[allow(clippy::missing_safety_doc)]
    pub unsafe extern "C" fn Java_com_anunknownalias_persephone_core_crypto_Except_except(
        mut env: JNIEnv,
        _: JClass,
        java_pattern: JString,
    ) -> jstring {
        let world = unsafe {
            rust_greeting(
                env.get_string(&java_pattern)
                    .expect("invalid pattern string")
                    .as_ptr(),
            )
        };
        let world_ptr = unsafe { CString::from_raw(world) };
        let output = env
            .new_string(world_ptr.to_str().unwrap())
            .expect("Couldn't create java string!");

        **output
    }
}

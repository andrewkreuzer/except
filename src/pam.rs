use std::ffi::{c_int, CStr};
use std::mem;
use std::{error::Error, ffi::CString};

unsafe extern "C" fn conversation(
    num_msg: c_int,
    pam_msg: *mut *const pam_sys::pam_message,
    pam_resp: *mut *mut pam_sys::pam_response,
    _appdata_ptr: *mut std::ffi::c_void,
) -> c_int {
    unsafe {
        let resp = libc::calloc(
            num_msg as usize,
            mem::size_of::<pam_sys::pam_response>() as libc::size_t,
        ) as *mut pam_sys::pam_response;

        println!("msgs: {:?}", num_msg);
        for i in 0..num_msg as isize {
            let msg_ptr = &mut *(*pam_msg.offset(i) as *mut pam_sys::pam_message);
            let resp_ptr = &mut *resp.offset(i);

            let msg = CStr::from_ptr(msg_ptr.msg);
            match msg_ptr.msg_style {
                pam_sys::PAM_PROMPT_ECHO_OFF => {
                    let password = rpassword::prompt_password("Please enter password: ").unwrap();
                    let c_token = CString::new(password.trim()).unwrap();
                    resp_ptr.resp = libc::strdup(c_token.as_ptr());
                }
                pam_sys::PAM_PROMPT_ECHO_ON => {
                    println!("{}", msg.to_str().unwrap());
                    let mut input_string = String::new();
                    std::io::stdin().read_line(&mut input_string).expect("Failed to read line");
                    let resp = CString::new(input_string.trim()).unwrap();
                    resp_ptr.resp = libc::strdup(resp.as_ptr());
                }
                pam_sys::PAM_TEXT_INFO => {
                    println!("{}", msg.to_str().unwrap());
                    let resp = CString::new("will do sir").unwrap();
                    resp_ptr.resp = libc::strdup(resp.as_ptr());
                }
                pam_sys::PAM_ERROR_MSG => {
                    println!("PAM_ERROR_MSG: {}", msg.to_str().unwrap());
                }
                _ => {
                    println!("Unknown message style: {:?}", msg_ptr.msg_style);
                    return pam_sys::PAM_CONV_ERR
                }
            }
        }

        *pam_resp = resp;
    }

    pam_sys::PAM_SUCCESS
}

pub fn pam_client() -> Result<(), Box<dyn Error>> {
    let service = CString::new("except")?;
    let user = CString::new("akreuzer")?;
    let mut handle: *mut pam_sys::pam_handle_t = std::ptr::null_mut();
    let conv: *const pam_sys::pam_conv = &pam_sys::pam_conv {
        conv: Some(conversation),
        appdata_ptr: std::ptr::null_mut(),
    };

    unsafe {
        let h = pam_sys::pam_start(service.as_ptr(), user.as_ptr(), conv, &mut handle);
        println!("start: {:?}", h == pam_sys::PAM_SUCCESS);
        let pamh = &mut *handle;

        let a = pam_sys::pam_authenticate(pamh, 0);
        println!("auth: {:?}", a == pam_sys::PAM_SUCCESS);

        let account = pam_sys::pam_acct_mgmt(handle, 0);
        println!("acct_mgmt: {:?}", account == pam_sys::PAM_SUCCESS);

        let e = pam_sys::pam_end(handle, 0);
        println!("end: {:?}", e == pam_sys::PAM_SUCCESS);
    }

    Ok(())
}

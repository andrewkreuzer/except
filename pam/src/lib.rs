use std::{
    ffi::{c_char, c_int},
    time::{Duration, Instant},
};

use log::{debug, error, trace, warn, LevelFilter};
use syslog::{BasicLogger, Facility, Formatter3164};

use except::ExceptManagerProxyBlocking;

// TODO: handle signals

fn send_msg<'a>(
    pamh: *const pam_sys::pam_handle_t,
    msg: &str,
    style: i32,
) -> Result<&'a str, Box<dyn std::error::Error>> {
    let mut pam_conv_ptr: *const libc::c_void = std::ptr::null();
    let ret = unsafe { pam_sys::pam_get_item(pamh, pam_sys::PAM_CONV, &mut pam_conv_ptr) };
    if ret != pam_sys::PAM_SUCCESS || pam_conv_ptr.is_null() {
        return Err(format!("Getting the pam conv failed: {}", ret).into());
    };

    let pam_conv = pam_conv_ptr as *const pam_sys::pam_conv;
    let pam_conv = unsafe { &*pam_conv };

    let msg = std::ffi::CString::new(msg)?;
    let mut pam_msg = pam_sys::pam_message {
        msg_style: style,
        msg: msg.as_ptr() as *const c_char,
    };
    let pam_msg_ptr = &mut (&mut pam_msg as *const pam_sys::pam_message);
    let resp: *mut i8 = std::ptr::null_mut();
    let mut pam_resp = pam_sys::pam_response {
        resp,
        resp_retcode: 0,
    };
    let pam_resp_ptr = &mut (&mut pam_resp as *mut pam_sys::pam_response);
    if let Some(conv) = pam_conv.conv {
        unsafe { conv(1, pam_msg_ptr, pam_resp_ptr, pam_conv.appdata_ptr) };
    }

    if pam_resp.resp.is_null() {
        return Err("Response is null".into());
    }

    let resp = unsafe { **pam_resp_ptr };
    let resp_msg = unsafe { std::ffi::CStr::from_ptr(resp.resp).to_str()? };

    Ok(resp_msg)
}

#[unsafe(no_mangle)]
#[allow(clippy::not_unsafe_ptr_arg_deref)] // pamh
pub extern "C" fn pam_sm_authenticate(
    pamh: *mut pam_sys::pam_handle_t,
    _flags: c_int,
    argc: c_int,
    argv: *const *const c_char,
) -> c_int {
    let mut ret = pam_sys::PAM_AUTH_ERR;
    if let Some(e) = logger().err() {
        error!("Failed to initialize logger: {}", e);
        return ret;
    };

    let args: Vec<Args> = parse_agrs(argc, argv).unwrap_or_default();
    if args.contains(&Args::Debug) {
        log::set_max_level(LevelFilter::Debug);
    }

    let msg = "Please login using your registered device...";
    let resp = match send_msg(pamh, msg, pam_sys::PAM_TEXT_INFO) {
        Ok(resp) => resp,
        Err(e) => &e.to_string(),
    };
    debug!("Login message response {:?}", resp);

    debug!("Starting pam_sm_authenticate with: {:?}", args);
    let connection = zbus::blocking::Connection::session().unwrap();
    let excpet_proxy = ExceptManagerProxyBlocking::new(&connection).unwrap();

    let max_tries = 3;
    let now = std::time::Instant::now();
    let timeout = Duration::from_secs(10);
    for _ in 0..max_tries {
        // TODO: handle multiple devices
        debug!("Getting default device");
        let id = match excpet_proxy.get_default_device() {
            Ok(id) => id,
            Err(e) => {
                debug!("Failed to get devices: {e}");
                break;
            }
        };

        debug!("Calling start_verify");
        if let Err(e) = excpet_proxy.start_verify(id) {
            debug!("Failed to start verify: {e}");
            break;
        }

        loop {
            debug!("Checking verify status");
            match excpet_proxy.verify_status() {
                Ok(true) => {
                    debug!("Verify status is true");
                    ret = pam_sys::PAM_SUCCESS;
                    break;
                }
                Ok(false) => {
                    trace!("Verify status is false");
                }
                Err(e) => {
                    debug!("Failed to get verify status: {e}");
                    break;
                }
            }

            if Instant::now() - now > timeout {
                warn!("Timeout waiting for verify");
                break;
            }

            std::thread::sleep(Duration::from_millis(200));
        }

        if ret == pam_sys::PAM_SUCCESS {
            break;
        }
    }

    if let Err(e) = excpet_proxy.stop_verify() {
        error!("Failed to stop verify: {e}");
    }

    ret
}

fn logger() -> Result<(), Box<dyn std::error::Error>> {
    let pid = unsafe { libc::getpid() as u32 };
    let formatter = Formatter3164 {
        facility: Facility::LOG_USER,
        hostname: None,
        process: "pam-except".into(),
        pid,
    };

    let logger = match syslog::unix(formatter) {
        Err(e) => return Err(Box::new(e)),
        Ok(logger) => logger,
    };

    log::set_boxed_logger(Box::new(BasicLogger::new(logger)))
        .map(|()| log::set_max_level(LevelFilter::Info))?;

    Ok(())
}

enum Args {
    Debug,
    UnknownArg(String),
}

impl std::fmt::Debug for Args {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Args::Debug => write!(f, "Debug"),
            Args::UnknownArg(s) => write!(f, "UnknownArg({})", s),
        }
    }
}

impl std::cmp::PartialEq for Args {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Args::Debug, Args::Debug) => true,
            (Args::UnknownArg(a), Args::UnknownArg(b)) => a == b,
            _ => false,
        }
    }
}

fn parse_agrs(
    argc: c_int,
    argv: *const *const c_char,
) -> Result<Vec<Args>, Box<dyn std::error::Error>> {
    let mut args: Vec<Args> = vec![];
    for i in 0..argc as isize {
        let arg = unsafe { *argv.offset(i) };
        let c_str = unsafe { std::ffi::CStr::from_ptr(arg) };
        let arg = match c_str.to_str()? {
            "debug" => Args::Debug,
            s => Args::UnknownArg(s.into()),
        };
        args.push(arg);
    }

    Ok(args)
}

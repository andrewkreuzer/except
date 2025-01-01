use std::error::Error;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use ring::aead::{Aad, BoundKey, Nonce, NonceSequence};
use ring::error::Unspecified;
use std::net::TcpStream;
use std::io::{Read, Write};

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
    pub unsafe extern "C" fn Java_com_anunknownalias_persephone_core_crypto_Except_call(
        _: JNIEnv,
        _: JClass,
    ) {
        call().unwrap();
    }

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

const KEY: &[u8; 32] = b"0123456789abcdef0123456789abcdef";
const CHALLENGE_REQUESTED: u8 = 80;
const CHALLENGE_ACCEPTED: u8 = 82;
// const CHALLENGE_CANCELLED: u8 = 127;
const CHALLENGE_APPROVED: u8 = 65;
// const CHALLENGE_REJECTED: u8 = 83;
const EOF: &[u8] = &[0; 4];
const FUNC1: fn(u8, u8) -> u8 = |op: u8, x: u8| x.wrapping_mul(op);

pub fn call() -> Result<(), Box<dyn Error>> {
    let mut stream = TcpStream::connect("192.168.2.106:6667")?;
    stream.write_all(&[CHALLENGE_REQUESTED])?;

    let mut challenge_buf = [0; 32];
    let mut length = 0;
    let mut reading = true;
    while reading {
        length += stream.read(&mut challenge_buf)?;
        if challenge_buf[..length].ends_with(EOF) {
            length -= EOF.len();
            reading = false;
        }
    }
    stream.write_all(&[CHALLENGE_ACCEPTED])?;

    let payload = &challenge_buf[..length];
    let _id = payload[0];
    let op = payload[1];
    let mut data = payload[2..].to_vec();
    let unbound_key = ring::aead::UnboundKey::new(&ring::aead::AES_256_GCM, KEY).unwrap();
    let mut opening_key = ring::aead::OpeningKey::new(unbound_key, NONCE_GEN);
    opening_key
        .open_in_place(Aad::empty(), &mut data)
        .unwrap();

    let mut response_data: Vec<u8> = data.iter().map(|x| FUNC1(op, *x)).collect();
    let unbound_key = ring::aead::UnboundKey::new(&ring::aead::AES_256_GCM, KEY).unwrap();
    let mut sealing_key = ring::aead::SealingKey::new(unbound_key, NONCE_GEN);
    sealing_key
        .seal_in_place_append_tag(Aad::empty(), &mut response_data)
        .unwrap();
    response_data.extend(EOF);
    stream.write_all(&response_data)?;

    let mut response = [0; 1];
    stream.read_exact(&mut response)?;

    if response[0] == CHALLENGE_APPROVED {
        print!("Challenge approved by: {}", stream.peer_addr()?.ip());
    } else {
        print!("Challenge rejected");
    }

    Ok(())
}

struct NonceGenerator {
    counter: [u8; 12],
}

impl NonceSequence for NonceGenerator {
    fn advance(&mut self) -> Result<Nonce, Unspecified> {
        // self.counter[0] = self.counter[0].wrapping_add(1);
        Ok(Nonce::assume_unique_for_key(self.counter))
    }
}

const NONCE_GEN: NonceGenerator = NonceGenerator { counter: [0u8; 12] };

#![allow(dead_code)]

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub fn sign_event(event_data: &str, secret: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .expect("HMAC can take key of any size");
    mac.update(event_data.as_bytes());
    let result = mac.finalize();
    BASE64.encode(result.into_bytes())
}

pub fn verify_signature(event_data: &str, signature: &str, secret: &str) -> bool {
    let expected = sign_event(event_data, secret);
    expected == signature
}

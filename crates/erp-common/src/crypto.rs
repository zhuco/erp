//! Small cryptographic helpers for adapter signing.
//!
//! We intentionally keep this module minimal:
//! - MD5 hex (upper/lower)
//! - HMAC-SHA256 hex (upper/lower)
//!
//! **Security note:** MD5 is used here only because several legacy e-commerce
//! platforms require it for request signing / webhook verification.

use hmac::{Hmac, Mac};
use sha2::Sha256;

/// MD5 digest as lowercase hex.
pub fn md5_hex_lower(data: &[u8]) -> String {
    format!("{:x}", md5::compute(data))
}

/// MD5 digest as uppercase hex.
pub fn md5_hex_upper(data: &[u8]) -> String {
    format!("{:X}", md5::compute(data))
}

/// HMAC-SHA256 digest as lowercase hex.
pub fn hmac_sha256_hex_lower(key: &[u8], data: &[u8]) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC can take key of any size");
    mac.update(data);
    let out = mac.finalize().into_bytes();
    hex::encode(out)
}

/// HMAC-SHA256 digest as uppercase hex.
pub fn hmac_sha256_hex_upper(key: &[u8], data: &[u8]) -> String {
    hmac_sha256_hex_lower(key, data).to_uppercase()
}

/// HMAC-MD5 digest as lowercase hex.
pub fn hmac_md5_hex_lower(key: &[u8], data: &[u8]) -> String {
    // Implement HMAC-MD5 manually to avoid depending on a Digest-compatible MD5 crate.
    // HMAC: MD5((K^opad) || MD5((K^ipad) || data))
    const BLOCK: usize = 64;

    // Step 1: Normalize key to block size.
    let mut k0 = [0u8; BLOCK];
    if key.len() > BLOCK {
        let d = md5::compute(key);
        k0[..16].copy_from_slice(&d.0);
    } else {
        k0[..key.len()].copy_from_slice(key);
    }

    // Step 2: Compute inner hash.
    let mut ipad = [0x36u8; BLOCK];
    let mut opad = [0x5cu8; BLOCK];
    for i in 0..BLOCK {
        ipad[i] ^= k0[i];
        opad[i] ^= k0[i];
    }

    let mut inner = Vec::with_capacity(BLOCK + data.len());
    inner.extend_from_slice(&ipad);
    inner.extend_from_slice(data);
    let inner_hash = md5::compute(inner);

    // Step 3: Compute outer hash.
    let mut outer = Vec::with_capacity(BLOCK + 16);
    outer.extend_from_slice(&opad);
    outer.extend_from_slice(&inner_hash.0);
    let outer_hash = md5::compute(outer);

    hex::encode(outer_hash.0)
}

/// HMAC-MD5 digest as uppercase hex.
pub fn hmac_md5_hex_upper(key: &[u8], data: &[u8]) -> String {
    hmac_md5_hex_lower(key, data).to_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_md5_known_vector() {
        // RFC 1321 test vector.
        assert_eq!(md5_hex_lower(b"abc"), "900150983cd24fb0d6963f7d28e17f72");
        assert_eq!(md5_hex_upper(b"abc"), "900150983CD24FB0D6963F7D28E17F72");
    }

    #[test]
    fn test_hmac_sha256_known_vector() {
        // Commonly-used test vector.
        let key = b"key";
        let msg = b"The quick brown fox jumps over the lazy dog";
        assert_eq!(
            hmac_sha256_hex_lower(key, msg),
            "f7bc83f430538424b13298e6aa6fb143ef4d59a14946175997479dbc2d1a3cd8"
        );
        assert_eq!(
            hmac_sha256_hex_upper(key, msg),
            "F7BC83F430538424B13298E6AA6FB143EF4D59A14946175997479DBC2D1A3CD8"
        );
    }

    #[test]
    fn test_hmac_md5_known_vector() {
        // RFC 2202 test vector (HMAC-MD5)
        let key = b"key";
        let msg = b"The quick brown fox jumps over the lazy dog";
        assert_eq!(
            hmac_md5_hex_lower(key, msg),
            "80070713463e7749b90c2dc24911e275"
        );
        assert_eq!(
            hmac_md5_hex_upper(key, msg),
            "80070713463E7749B90C2DC24911E275"
        );
    }
}

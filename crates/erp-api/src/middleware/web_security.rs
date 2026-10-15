//! Public CORS is an exact HTTPS allow-list. Development Any is not that list.

use erp_domain::receipt::{ReceiptError, admit_public_origin};

/// A public deployment refuses a wildcard and any origin that is not on the stored list.
pub fn public_cors_allows(origin: &str, allowed: &[String]) -> Result<(), ReceiptError> {
    admit_public_origin(origin, allowed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_cors_refuses_wildcard_and_http() {
        let allowed = vec!["https://shop.example".to_string()];
        assert!(public_cors_allows("https://shop.example", &allowed).is_ok());
        assert!(public_cors_allows("*", &allowed).is_err());
        assert!(public_cors_allows("http://shop.example", &allowed).is_err());
        assert!(public_cors_allows("https://other.example", &allowed).is_err());
    }
}

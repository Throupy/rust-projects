use hmac::{Hmac, Mac};
use sha2::Sha256;
use subtle::ConstantTimeEq;

type HmacSha256 = Hmac<Sha256>;

// only a dev project
// TODO: Secrets management, to learn this.
// There are always env vars, but what's rust best practice?
const PSK: &str = "super-secret-key";

pub fn compute_hmac(payload: &str) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(PSK.as_bytes()).unwrap();
    mac.update(payload.as_bytes());
    mac.finalize().into_bytes().to_vec()
}

pub fn sign(sender: &str, content: &str) -> String {
    // compute username:content
    let to_sign = format!("{}:{}", sender, content);
    hex::encode(compute_hmac(&to_sign))
}

pub fn verify(sender: &str, content: &str, signature: &str) -> bool {
    // payload is format: username:content, construct it
    let payload: String = format!("{}:{}", sender, content);
    let expected = compute_hmac(payload.as_str());
    let provided = match hex::decode(signature) {
        Ok(b) => b,
        Err(_) => return false,
    };
    
    expected.ct_eq(&provided).into()
}

#[cfg(test)]
mod tests {
    use super::{compute_hmac, sign, verify};

    const VECTOR_SENDER: &str = "owen";
    const VECTOR_CONTENT: &str = "hello, world";
    const VECTOR_PAYLOAD: &str = "owen:hello, world";

    #[test]
    fn test_print_hmac() {
        let signature = sign(VECTOR_SENDER, VECTOR_CONTENT);
        println!("Using '{}' as vector for tests", signature)
    }

    #[test]
    fn test_sign_produces_known_value() {
        let signature = sign(VECTOR_SENDER, VECTOR_CONTENT);
        assert_eq!(&signature, "8ffbae07308ebc4d060db69887284787d4674c87c2e6b448e869e139720becea");
    }
    
    #[test]
    fn test_verify_valid_signature() {
        let signature = sign(VECTOR_SENDER, VECTOR_CONTENT);
        assert!(verify(&VECTOR_SENDER, &VECTOR_CONTENT, &signature))
    }

    #[test]
    fn test_verify_tampered_content() {
        let signature = sign(VECTOR_SENDER, "goodbye, world");
        assert!(!verify(&VECTOR_SENDER, &VECTOR_CONTENT, &signature))
    }

    #[test]
    fn test_verify_tampered_sender() {
        let signature = sign("differentuser", VECTOR_CONTENT);
        assert!(!verify(&VECTOR_SENDER, &VECTOR_CONTENT, &signature))
    }

    #[test]
    fn test_verify_invalid_hex() {
        assert!(!verify(&VECTOR_SENDER, &VECTOR_CONTENT, "inv4lid-h3x!!"))
    }

    #[test]
    fn test_compute_hmac_produces_known_bytes() {
        let result = compute_hmac(&VECTOR_PAYLOAD);
        let hex_result = hex::encode(&result);
        assert_eq!(hex_result, "8ffbae07308ebc4d060db69887284787d4674c87c2e6b448e869e139720becea");
    }
}

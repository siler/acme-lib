use once_cell::sync::Lazy;
use openssl::{
    ec::{Asn1Flag, EcGroup, EcKey},
    hash::MessageDigest,
    nid::Nid,
    pkey::{self, PKey},
    rsa::Rsa,
    stack::Stack,
    x509::{extension::SubjectAlternativeName, X509Req, X509ReqBuilder, X509},
};
use time::{macros::format_description, OffsetDateTime, PrimitiveDateTime};

use crate::Result;

pub(crate) static EC_GROUP_P256: Lazy<EcGroup> = Lazy::new(|| ec_group(Nid::X9_62_PRIME256V1));
pub(crate) static EC_GROUP_P384: Lazy<EcGroup> = Lazy::new(|| ec_group(Nid::SECP384R1));

fn ec_group(nid: Nid) -> EcGroup {
    let mut g = EcGroup::from_curve_name(nid).expect("EcGroup");
    // this is required for openssl 1.0.x (but not 1.1.x)
    g.set_asn1_flag(Asn1Flag::NAMED_CURVE);
    g
}

/// Make an RSA private key (from which we can derive a public key).
///
/// This library does not check the number of bits used to create the key pair.
/// For Let's Encrypt, the bits must be between 2048 and 4096.
pub fn create_rsa_key(bits: u32) -> PKey<pkey::Private> {
    let pri_key_rsa = Rsa::generate(bits).expect("Rsa::generate");
    PKey::from_rsa(pri_key_rsa).expect("from_rsa")
}

/// Make a P-256 private key (from which we can derive a public key).
pub fn create_p256_key() -> PKey<pkey::Private> {
    let pri_key_ec = EcKey::generate(&EC_GROUP_P256).expect("EcKey");
    PKey::from_ec_key(pri_key_ec).expect("from_ec_key")
}

/// Make a P-384 private key pair (from which we can derive a public key).
pub fn create_p384_key() -> PKey<pkey::Private> {
    let pri_key_ec = EcKey::generate(&EC_GROUP_P384).expect("EcKey");
    PKey::from_ec_key(pri_key_ec).expect("from_ec_key")
}

pub(crate) fn create_csr(pkey: &PKey<pkey::Private>, domains: &[&str]) -> Result<X509Req> {
    //
    // the csr builder
    let mut req_bld = X509ReqBuilder::new().expect("X509ReqBuilder");

    // set private/public key in builder
    req_bld.set_pubkey(pkey).expect("set_pubkey");

    // set all domains as alt names
    let mut stack = Stack::new().expect("Stack::new");
    let ctx = req_bld.x509v3_context(None);
    let as_lst = domains
        .iter()
        .map(|&e| format!("DNS:{}", e))
        .collect::<Vec<_>>()
        .join(", ");
    let as_lst = as_lst[4..].to_string();
    let mut an = SubjectAlternativeName::new();
    an.dns(&as_lst);
    let ext = an.build(&ctx).expect("SubjectAlternativeName::build");
    stack.push(ext).expect("Stack::push");
    req_bld.add_extensions(&stack).expect("add_extensions");

    // sign it
    req_bld
        .sign(pkey, MessageDigest::sha256())
        .expect("csr_sign");

    // the csr
    Ok(req_bld.build())
}

/// Encapsulated certificate and private key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Certificate {
    private_key: String,
    certificate: String,
}

impl Certificate {
    pub(crate) fn new(private_key: String, certificate: String) -> Self {
        Certificate {
            private_key,
            certificate,
        }
    }

    /// The PEM encoded private key.
    pub fn private_key(&self) -> &str {
        &self.private_key
    }

    /// The private key as DER.
    pub fn private_key_der(&self) -> Vec<u8> {
        let pkey = PKey::private_key_from_pem(self.private_key.as_bytes()).expect("from_pem");
        pkey.private_key_to_der().expect("private_key_to_der")
    }

    /// The PEM encoded issued certificate.
    pub fn certificate(&self) -> &str {
        &self.certificate
    }

    /// The issued certificate as DER.
    pub fn certificate_der(&self) -> Vec<u8> {
        let x509 = X509::from_pem(self.certificate.as_bytes()).expect("from_pem");
        x509.to_der().expect("to_der")
    }

    /// Inspect the certificate to count the number of (whole) valid days left.
    ///
    /// It's up to the ACME API provider to decide how long an issued certificate is valid.
    /// Let's Encrypt sets the validity to 90 days. This function reports 89 days for newly
    /// issued cert, since it counts _whole_ days.
    ///
    /// It is possible to get negative days for an expired certificate.
    pub fn valid_days_left(&self) -> i64 {
        // the cert used in the tests is not valid to load as x509
        if cfg!(test) {
            return 89;
        }

        // load as x509
        let x509 = X509::from_pem(self.certificate.as_bytes()).expect("from_pem");

        // convert asn1 time to Tm
        let not_after = format!("{}", x509.not_after());
        // Display trait produces this format, which is kinda dumb.
        // Apr 19 08:48:46 2019 GMT
        let expires = parse_date(&not_after);
        let dur = expires - OffsetDateTime::now_utc();

        dur.whole_days()
    }
}

fn parse_date(s: &str) -> OffsetDateTime {
    debug!("Parse date/time: {}", s);
    let format = format_description!(
        "[month repr:short] [day padding:space] [hour repr:24]:[minute]:[second] [year repr:full] GMT"
    );
    PrimitiveDateTime::parse(s, &format).unwrap().assume_utc()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_date() {
        let x = parse_date("May  3 07:40:15 2019 GMT");
        let format = format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");
        assert_eq!(x.format(format).unwrap(), "2019-05-03 07:40:15");
    }
}

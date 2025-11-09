use anyhow::{anyhow, Context, Result};
use dirs::home_dir;
use rcgen::{BasicConstraints, Certificate, CertificateParams, DistinguishedName, DnType, IsCa, KeyPair, SanType, SerialNumber};
use rustls_pki_types::{CertificateDer, PrivateKeyDer}; // reserved for future use
use std::{fs, path::{Path, PathBuf}};

pub struct CaMaterial {
    pub ca_cert_pem: String,
    pub ca: Certificate,
}

pub fn ca_default_dir() -> PathBuf {
    if let Some(home) = home_dir() { home.join(".config/span/ca") } else { PathBuf::from("./.span/ca") }
}

pub fn load_or_init_ca(dir: Option<&Path>) -> Result<CaMaterial> {
    let _owned;
    let dir: &Path = match dir {
        Some(p) => p,
        None => { _owned = ca_default_dir(); _owned.as_path() }
    };
    fs::create_dir_all(dir).ok();
    let cert_path = dir.join("ca.crt");
    let key_path = dir.join("ca.key");

    if cert_path.exists() && key_path.exists() {
        let cert_pem = fs::read_to_string(&cert_path)?;
        let key_pem = fs::read_to_string(&key_path)?;
        let key = KeyPair::from_pem(&key_pem).context("invalid CA key pem")?;
        let mut params = CertificateParams::default();
        params.alg = &rcgen::PKCS_ECDSA_P256_SHA256;
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, "Span Root CA");
        params.distinguished_name = dn;
        params.key_pair = Some(key);
        // Note: rcgen doesn't reconstruct from an existing cert; we rebuild params with same subject and key
        let ca = Certificate::from_params(params).context("build CA from existing key")?;
        return Ok(CaMaterial { ca_cert_pem: cert_pem, ca });
    }

    let mut params = CertificateParams::default();
    params.alg = &rcgen::PKCS_ECDSA_P256_SHA256;
    params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, "Span Root CA");
    params.distinguished_name = dn;
    params.not_before = rcgen::date_time_ymd(2020, 1, 1);
    params.not_after = rcgen::date_time_ymd(2050, 1, 1);

    let ca = Certificate::from_params(params)?;
    let cert_pem = ca.serialize_pem()?;
    let key_pem = ca.serialize_private_key_pem();

    fs::write(&cert_path, &cert_pem)?;
    fs::write(&key_path, &key_pem)?;

    Ok(CaMaterial { ca_cert_pem: cert_pem, ca })
}

pub fn generate_node_cert(node_id: &str, ca: &Certificate) -> Result<(String, String)> {
    let mut params = CertificateParams::new(vec![node_id.to_string()]);
    params.alg = &rcgen::PKCS_ECDSA_P256_SHA256;
    params.not_before = rcgen::date_time_ymd(2020, 1, 1);
    // rcgen 0.12 doesn't support building from SystemTime; set a conservative expiry.
    params.not_after = rcgen::date_time_ymd(2035, 1, 1);
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, format!("node-{}", node_id));
    params.distinguished_name = dn;
    params.serial_number = Some(SerialNumber::from(rand::random::<u64>()));
    params.subject_alt_names = vec![SanType::DnsName(node_id.into())];

    let cert = Certificate::from_params(params)?;
    let cert_pem = cert.serialize_pem_with_signer(ca)?;
    let key_pem = cert.serialize_private_key_pem();
    Ok((cert_pem, key_pem))
}

pub fn load_identity_from_pem(cert_pem: &str, key_pem: &str) -> Result<(Vec<u8>, Vec<u8>)> {
    Ok((cert_pem.as_bytes().to_vec(), key_pem.as_bytes().to_vec()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn node_cert_contains_san() {
        let ca = load_or_init_ca(None).expect("ca");
        let node_id = "123e4567-e89b-12d3-a456-426614174000";
        let (cert_pem, _key) = generate_node_cert(node_id, &ca.ca).expect("node cert");
        // Parse PEM and extract DER using x509-parser's pem helper
        let (_rem, pem) = x509_parser::pem::parse_x509_pem(cert_pem.as_bytes()).expect("pem");
        let (_rem, parsed) = x509_parser::parse_x509_certificate(&pem.contents).expect("x509");
        if let Ok(Some(san_ext)) = parsed.subject_alternative_name() {
            let names = &san_ext.value.general_names;
            let has = names.iter().any(|gn| {
                if let x509_parser::extensions::GeneralName::DNSName(dns) = gn {
                    let s: &str = dns.as_ref();
                    s == node_id
                } else { false }
            });
            assert!(has);
        } else {
            panic!("san");
        }
    }
}

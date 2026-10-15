//! Old OpenSSL-encrypted PEM keys ("Proc-Type: 4,ENCRYPTED" with a
//! "DEK-Info" header), as made by `openssl genrsa -des3` and some cloud
//! consoles. russh only reads the AES-128-CBC variant, so these are
//! decrypted here, in memory, and handed back as an unencrypted PEM string.
//! Nothing is written to disk.

use base64::{Engine, engine::general_purpose::STANDARD};
use cbc::cipher::{BlockModeDecrypt, KeyIvInit, block_padding::Pkcs7};
use md5::{Digest, Md5};

#[derive(Debug, PartialEq, Eq)]
pub enum LegacyPemError {
    /// The key is encrypted and no passphrase was given.
    NeedsPassphrase,
    /// The passphrase didn't decrypt the key.
    WrongPassphrase,
    /// An encryption this app doesn't read.
    Unsupported(String),
    Malformed,
}

/// Returns `Ok(None)` when `text` isn't a legacy encrypted PEM key.
pub fn decrypt(text: &str, passphrase: Option<&str>) -> Result<Option<String>, LegacyPemError> {
    let mut lines = text.lines().map(str::trim);
    let Some(begin) = lines.find(|l| l.starts_with("-----BEGIN ")) else {
        return Ok(None);
    };
    let label = begin
        .trim_start_matches("-----BEGIN ")
        .trim_end_matches("-----")
        .to_owned();
    let mut encrypted = false;
    let mut dek_info = None;
    let mut body = String::new();
    for line in lines {
        if line.starts_with("-----END ") {
            break;
        } else if line == "Proc-Type: 4,ENCRYPTED" {
            encrypted = true;
        } else if let Some(info) = line.strip_prefix("DEK-Info:") {
            dek_info = Some(info.trim().to_owned());
        } else if !line.contains(':') {
            body.push_str(line);
        }
    }
    let Some(dek_info) = dek_info.filter(|_| encrypted) else {
        return Ok(None);
    };
    let passphrase = passphrase
        .filter(|p| !p.is_empty())
        .ok_or(LegacyPemError::NeedsPassphrase)?;
    let (cipher, iv_hex) = dek_info.split_once(',').ok_or(LegacyPemError::Malformed)?;
    let iv = decode_hex(iv_hex.trim()).ok_or(LegacyPemError::Malformed)?;
    let data = STANDARD
        .decode(body.as_bytes())
        .map_err(|_| LegacyPemError::Malformed)?;
    if iv.len() < 8 {
        return Err(LegacyPemError::Malformed);
    }
    let decrypted = match cipher {
        "DES-EDE3-CBC" => run::<cbc::Decryptor<des::TdesEde3>>(&data, passphrase, &iv, 24),
        "AES-128-CBC" => run::<cbc::Decryptor<aes::Aes128>>(&data, passphrase, &iv, 16),
        "AES-192-CBC" => run::<cbc::Decryptor<aes::Aes192>>(&data, passphrase, &iv, 24),
        "AES-256-CBC" => run::<cbc::Decryptor<aes::Aes256>>(&data, passphrase, &iv, 32),
        other => return Err(LegacyPemError::Unsupported(other.to_owned())),
    }?;
    // The plaintext is DER: a SEQUENCE. Anything else means a wrong passphrase
    // that happened to leave valid padding.
    if decrypted.first() != Some(&0x30) {
        return Err(LegacyPemError::WrongPassphrase);
    }
    let encoded = STANDARD.encode(decrypted);
    let mut pem = format!("-----BEGIN {label}-----\n");
    for chunk in encoded.as_bytes().chunks(64) {
        pem.push_str(std::str::from_utf8(chunk).unwrap_or_default());
        pem.push('\n');
    }
    pem.push_str(&format!("-----END {label}-----\n"));
    Ok(Some(pem))
}

fn run<D: KeyIvInit + BlockModeDecrypt>(
    data: &[u8],
    passphrase: &str,
    iv: &[u8],
    key_len: usize,
) -> Result<Vec<u8>, LegacyPemError> {
    let key = bytes_to_key(passphrase.as_bytes(), &iv[..8], key_len);
    D::new_from_slices(&key, iv)
        .map_err(|_| LegacyPemError::Malformed)?
        .decrypt_padded_vec::<Pkcs7>(data)
        .map_err(|_| LegacyPemError::WrongPassphrase)
}

/// OpenSSL's EVP_BytesToKey with MD5 and one round, as used by these keys.
fn bytes_to_key(passphrase: &[u8], salt: &[u8], len: usize) -> Vec<u8> {
    let mut key = Vec::with_capacity(len + 16);
    let mut previous: Vec<u8> = Vec::new();
    while key.len() < len {
        let mut hasher = Md5::new();
        hasher.update(&previous);
        hasher.update(passphrase);
        hasher.update(salt);
        previous = hasher.finalize().to_vec();
        key.extend_from_slice(&previous);
    }
    key.truncate(len);
    key
}

fn decode_hex(hex: &str) -> Option<Vec<u8>> {
    if !hex.len().is_multiple_of(2) {
        return None;
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Throwaway test keys, passphrase "secret".
    const DES3: &str = include_str!("testdata/des3.pem");
    const AES256: &str = include_str!("testdata/aes256.pem");

    #[test]
    fn decrypts_des3_and_aes256_keys_that_russh_then_reads() {
        for key in [DES3, AES256] {
            let pem = decrypt(key, Some("secret")).unwrap().unwrap();
            assert!(pem.starts_with("-----BEGIN RSA PRIVATE KEY-----\n"));
            russh::keys::decode_secret_key(&pem, None).unwrap();
        }
    }

    #[test]
    fn asks_for_a_passphrase_and_rejects_a_wrong_one() {
        assert_eq!(decrypt(DES3, None), Err(LegacyPemError::NeedsPassphrase));
        assert_eq!(
            decrypt(DES3, Some("")),
            Err(LegacyPemError::NeedsPassphrase)
        );
        assert_eq!(
            decrypt(AES256, Some("wrong")),
            Err(LegacyPemError::WrongPassphrase)
        );
    }

    #[test]
    fn leaves_other_keys_alone() {
        let plain =
            "-----BEGIN OPENSSH PRIVATE KEY-----\nAAAA\n-----END OPENSSH PRIVATE KEY-----\n";
        assert_eq!(decrypt(plain, Some("x")), Ok(None));
        assert_eq!(decrypt("not a key", None), Ok(None));
    }
}

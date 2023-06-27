use rand::Rng;
use ring::digest::{Context, SHA1_FOR_LEGACY_USE_ONLY};
use serde::Serialize;

#[derive(Serialize)]
struct EncryptionData {
    ct: String,
    iv: String,
    s: String,
}

pub fn encrypt(data: &str, key: &str) -> String {
    let enc_data = aes_encrypt(data, key).expect("encryption failed");
    let enc_data_json = serde_json::to_string(&enc_data).expect("JSON serialization failed");
    enc_data_json
}

fn aes_encrypt(content: &str, password: &str) -> Result<EncryptionData, &'static str> {
    let salt: Vec<u8> = rand::thread_rng().gen::<[u8; 8]>().to_vec();
    let (key, iv) = default_evp_kdf(password.as_bytes(), &salt)?;

    use aes::cipher::{block_padding::Pkcs7, BlockEncryptMut, KeyIvInit};
    type Aes256CbcEnc = cbc::Encryptor<aes::Aes256>;

    let mut buf = [0u8; 4096];

    let cipher_bytes = Aes256CbcEnc::new_from_slices(&key, &iv)
        .unwrap()
        .encrypt_padded_b2b_mut::<Pkcs7>(content.as_bytes(), &mut buf)
        .unwrap();

    let mut md5_hash = Context::new(&SHA1_FOR_LEGACY_USE_ONLY);
    let mut salted = String::new();
    let mut dx: Vec<u8> = vec![];

    for _ in 0..3 {
        md5_hash.update(&dx);
        md5_hash.update(password.as_bytes());
        md5_hash.update(&salt);

        let digest = md5_hash.finish();
        dx = digest.as_ref().to_vec();
        md5_hash = Context::new(&SHA1_FOR_LEGACY_USE_ONLY);

        salted += &hex::encode(&dx);
    }
    #[allow(deprecated)]
    let cipher_text = base64::encode(&cipher_bytes);
    let enc_data = EncryptionData {
        ct: cipher_text,
        iv: salted[64..64 + 32].to_string(),
        s: hex::encode(&salt),
    };
    Ok(enc_data)
}

fn evp_kdf(
    password: &[u8],
    salt: &[u8],
    key_size: usize,
    iterations: usize,
    hash_algorithm: &str,
) -> Result<Vec<u8>, &'static str> {
    let mut derived_key_bytes = Vec::new();
    match hash_algorithm {
        "md5" => {
            let mut block = Vec::new();
            let mut hasher = Context::new(&SHA1_FOR_LEGACY_USE_ONLY);

            while derived_key_bytes.len() < key_size * 4 {
                if !block.is_empty() {
                    hasher.update(&block);
                }
                hasher.update(password);
                hasher.update(salt);

                let digest = hasher.finish();
                block = digest.as_ref().to_vec();
                hasher = Context::new(&SHA1_FOR_LEGACY_USE_ONLY);

                for _ in 1..iterations {
                    hasher.update(&block);
                    let digest = hasher.finish();
                    block = digest.as_ref().to_vec();
                    hasher = Context::new(&SHA1_FOR_LEGACY_USE_ONLY);
                }

                derived_key_bytes.extend_from_slice(&block);
            }
        }
        _ => return Err("unsupported hash algorithm"),
    }

    Ok(derived_key_bytes[..key_size * 4].to_vec())
}

fn default_evp_kdf(password: &[u8], salt: &[u8]) -> Result<(Vec<u8>, Vec<u8>), &'static str> {
    let key_size = 256 / 32;
    let iv_size = 128 / 32;
    let derived_key_bytes = evp_kdf(password, salt, key_size + iv_size, 1, "md5")?;
    Ok((
        derived_key_bytes[..key_size * 4].to_vec(),
        derived_key_bytes[key_size * 4..].to_vec(),
    ))
}

use crate::utils::b16_deserialize;
use crate::utils::b16_serialize;
use crate::utils::b24_deserialize;
use crate::utils::b24_serialize;
use crate::utils::b32_deserialize;
use crate::utils::b32_serialize;
use crate::utils::vec_deserialize;
use crate::utils::vec_serialize;
use serde::{Deserialize, Serialize};
use sha3::Digest;
use std::convert::TryInto;
use std::io::ErrorKind;
use std::result::Result;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

#[cfg(feature = "use_openssl")]
use openssl::symm::Cipher;

#[cfg(not(feature = "enable_openssl"))]
use ctr::cipher::*;
#[cfg(not(feature = "enable_openssl"))]
type Aes128Ctr = ctr::Ctr128BE<aes::Aes128>;
#[cfg(not(feature = "enable_openssl"))]
type Aes192Ctr = ctr::Ctr128BE<aes::Aes192>;
#[cfg(not(feature = "enable_openssl"))]
type Aes256Ctr = ctr::Ctr128BE<aes::Aes256>;

use super::*;

/// Represents an encryption key that will give confidentiality to
/// data stored within the redo-log. Note this does not give integrity
/// which comes from the `PrivateKey` crypto instead.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum EncryptKey {
    Aes128(
        #[serde(serialize_with = "b16_serialize", deserialize_with = "b16_deserialize")] [u8; 16],
    ),
    Aes192(
        #[serde(serialize_with = "b24_serialize", deserialize_with = "b24_deserialize")] [u8; 24],
    ),
    Aes256(
        #[serde(serialize_with = "b32_serialize", deserialize_with = "b32_deserialize")] [u8; 32],
    ),
}

impl EncryptKey {
    pub fn generate(size: KeySize) -> EncryptKey {
        RandomGeneratorAccessor::generate_encrypt_key(size)
    }

    pub fn resize(&self, size: KeySize) -> EncryptKey {
        // Pad the current key out to 256 bytes (with zeros)
        let mut bytes = self.value().iter().map(|a| *a).collect::<Vec<_>>();
        while bytes.len() < 32 {
            bytes.push(0u8);
        }

        // Build a new key from the old key using these bytes
        match size {
            KeySize::Bit128 => {
                let aes_key: [u8; 16] = bytes
                    .into_iter()
                    .take(16)
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap();
                EncryptKey::Aes128(aes_key)
            }
            KeySize::Bit192 => {
                let aes_key: [u8; 24] = bytes
                    .into_iter()
                    .take(24)
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap();
                EncryptKey::Aes192(aes_key)
            }
            KeySize::Bit256 => {
                let aes_key: [u8; 32] = bytes
                    .into_iter()
                    .take(32)
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap();
                EncryptKey::Aes256(aes_key)
            }
        }
    }

    pub fn size(&self) -> KeySize {
        match self {
            EncryptKey::Aes128(_) => KeySize::Bit128,
            EncryptKey::Aes192(_) => KeySize::Bit192,
            EncryptKey::Aes256(_) => KeySize::Bit256,
        }
    }

    pub fn value(&self) -> &[u8] {
        match self {
            EncryptKey::Aes128(a) => a,
            EncryptKey::Aes192(a) => a,
            EncryptKey::Aes256(a) => a,
        }
    }

    #[cfg(feature = "enable_openssl")]
    pub fn cipher(&self) -> Cipher {
        match self.size() {
            KeySize::Bit128 => Cipher::aes_128_ctr(),
            KeySize::Bit192 => Cipher::aes_192_ctr(),
            KeySize::Bit256 => Cipher::aes_256_ctr(),
        }
    }

    #[cfg(feature = "enable_openssl")]
    pub fn encrypt_with_iv(&self, iv: &InitializationVector, data: &[u8]) -> Vec<u8> {
        let mut iv_store;
        let iv = match iv.bytes.len() {
            16 => iv,
            _ => {
                iv_store = InitializationVector {
                    bytes: iv.bytes.clone().into_iter().take(16).collect::<Vec<_>>(),
                };
                while iv_store.bytes.len() < 16 {
                    iv_store.bytes.push(0u8);
                }
                &iv_store
            }
        };
        openssl::symm::encrypt(self.cipher(), self.value(), Some(&iv.bytes[..]), data).unwrap()
    }

    #[cfg(not(feature = "enable_openssl"))]
    pub fn encrypt_with_iv(&self, iv: &InitializationVector, data: &[u8]) -> Vec<u8> {
        let mut iv_store;
        let iv = match iv.bytes.len() {
            16 => iv,
            _ => {
                iv_store = InitializationVector {
                    bytes: iv.bytes.clone().into_iter().take(16).collect::<Vec<_>>(),
                };
                while iv_store.bytes.len() < 16 {
                    iv_store.bytes.push(0u8);
                }
                &iv_store
            }
        };

        let mut data = data.to_vec();

        match self.size() {
            KeySize::Bit128 => {
                let mut cipher = Aes128Ctr::new(self.value().into(), (&iv.bytes[..]).into());
                cipher.apply_keystream(data.as_mut_slice());
            }
            KeySize::Bit192 => {
                let mut cipher = Aes192Ctr::new(self.value().into(), (&iv.bytes[..]).into());
                cipher.apply_keystream(data.as_mut_slice());
            }
            KeySize::Bit256 => {
                let mut cipher = Aes256Ctr::new(self.value().into(), (&iv.bytes[..]).into());
                cipher.apply_keystream(data.as_mut_slice());
            }
        }

        data
    }

    pub fn encrypt(&self, data: &[u8]) -> EncryptResult {
        let iv = InitializationVector::generate();
        let data = self.encrypt_with_iv(&iv, data);
        EncryptResult { iv: iv, data: data }
    }

    #[cfg(feature = "enable_openssl")]
    pub fn decrypt(&self, iv: &InitializationVector, data: &[u8]) -> Vec<u8> {
        let mut iv_store;
        let iv = match iv.bytes.len() {
            16 => iv,
            _ => {
                iv_store = InitializationVector {
                    bytes: iv.bytes.clone().into_iter().take(16).collect::<Vec<_>>(),
                };
                while iv_store.bytes.len() < 16 {
                    iv_store.bytes.push(0u8);
                }
                &iv_store
            }
        };
        openssl::symm::decrypt(self.cipher(), self.value(), Some(&iv.bytes[..]), data).unwrap()
    }

    #[cfg(not(feature = "enable_openssl"))]
    pub fn decrypt(&self, iv: &InitializationVector, data: &[u8]) -> Vec<u8> {
        let mut iv_store;
        let iv = match iv.bytes.len() {
            16 => iv,
            _ => {
                iv_store = InitializationVector {
                    bytes: iv.bytes.clone().into_iter().take(16).collect::<Vec<_>>(),
                };
                while iv_store.bytes.len() < 16 {
                    iv_store.bytes.push(0u8);
                }
                &iv_store
            }
        };

        let mut data = data.to_vec();

        match self.size() {
            KeySize::Bit128 => {
                let mut cipher = Aes128Ctr::new(self.value().into(), (&iv.bytes[..]).into());
                cipher.apply_keystream(data.as_mut_slice());
            }
            KeySize::Bit192 => {
                let mut cipher = Aes192Ctr::new(self.value().into(), (&iv.bytes[..]).into());
                cipher.apply_keystream(data.as_mut_slice());
            }
            KeySize::Bit256 => {
                let mut cipher = Aes256Ctr::new(self.value().into(), (&iv.bytes[..]).into());
                cipher.apply_keystream(data.as_mut_slice());
            }
        }

        data
    }

    #[allow(dead_code)]
    pub fn as_bytes(&self) -> Vec<u8> {
        Vec::from(self.value())
    }

    #[allow(dead_code)]
    pub fn from_bytes(bytes: &[u8]) -> Result<EncryptKey, std::io::Error> {
        let bytes: Vec<u8> = Vec::from(bytes);
        match bytes.len() {
            16 => {
                Ok(EncryptKey::Aes128(bytes.try_into().expect(
                    "Internal error while deserializing the Encryption Key",
                )))
            }
            24 => {
                Ok(EncryptKey::Aes192(bytes.try_into().expect(
                    "Internal error while deserializing the Encryption Key",
                )))
            }
            32 => {
                Ok(EncryptKey::Aes256(bytes.try_into().expect(
                    "Internal error while deserializing the Encryption Key",
                )))
            }
            _ => Result::Err(std::io::Error::new(
                ErrorKind::Other,
                format!(
                    "The encryption key bytes are the incorrect length ({}).",
                    bytes.len()
                ),
            )),
        }
    }

    pub fn hash(&self) -> AteHash {
        match &self {
            EncryptKey::Aes128(a) => AteHash::from_bytes(a),
            EncryptKey::Aes192(a) => AteHash::from_bytes(a),
            EncryptKey::Aes256(a) => AteHash::from_bytes(a),
        }
    }

    pub fn short_hash(&self) -> ShortHash {
        match &self {
            EncryptKey::Aes128(a) => ShortHash::from_bytes(a),
            EncryptKey::Aes192(a) => ShortHash::from_bytes(a),
            EncryptKey::Aes256(a) => ShortHash::from_bytes(a),
        }
    }

    pub fn from_seed_string(str: String, size: KeySize) -> EncryptKey {
        EncryptKey::from_seed_bytes(str.as_bytes(), size)
    }

    pub fn from_seed_bytes(seed_bytes: &[u8], size: KeySize) -> EncryptKey {
        let mut hasher = sha3::Keccak384::new();
        hasher.update(seed_bytes);
        let result = hasher.finalize();

        match size {
            KeySize::Bit128 => {
                let aes_key: [u8; 16] = result
                    .into_iter()
                    .take(16)
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap();
                EncryptKey::Aes128(aes_key)
            }
            KeySize::Bit192 => {
                let aes_key: [u8; 24] = result
                    .into_iter()
                    .take(24)
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap();
                EncryptKey::Aes192(aes_key)
            }
            KeySize::Bit256 => {
                let aes_key: [u8; 32] = result
                    .into_iter()
                    .take(32)
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap();
                EncryptKey::Aes256(aes_key)
            }
        }
    }

    pub fn xor(ek1: &EncryptKey, ek2: &EncryptKey) -> EncryptKey {
        let mut ek1_bytes = ek1.as_bytes();
        let ek2_bytes = ek2.as_bytes();

        ek1_bytes
            .iter_mut()
            .zip(ek2_bytes.iter())
            .for_each(|(x1, x2)| *x1 ^= *x2);

        EncryptKey::from_bytes(&ek1_bytes[..])
            .expect("Internal error while attempting to XOR encryption keys")
    }
}

impl std::fmt::Display for EncryptKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EncryptKey::Aes128(a) => write!(f, "aes-128:{}", hex::encode(a)),
            EncryptKey::Aes192(a) => write!(f, "aes-192:{}", hex::encode(a)),
            EncryptKey::Aes256(a) => write!(f, "aes-256:{}", hex::encode(a)),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct EncryptResult {
    pub iv: InitializationVector,
    #[serde(serialize_with = "vec_serialize", deserialize_with = "vec_deserialize")]
    pub data: Vec<u8>,
}

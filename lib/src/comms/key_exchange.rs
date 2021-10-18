#![allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use error_chain::bail;
use tokio::io::{ AsyncReadExt, AsyncWriteExt};
use crate::crypto::{EncryptKey, PublicEncryptKey, InitializationVector};

use crate::error::*;
use crate::crypto::KeySize;
use crate::crypto::PrivateEncryptKey;

use super::StreamRx;
use super::StreamTx;
use super::CertificateValidation;

#[cfg(feature = "enable_client")]
pub(super) async fn mesh_key_exchange_sender(stream_rx: &mut StreamRx, stream_tx: &mut StreamTx, key_size: KeySize, validation: CertificateValidation) -> Result<EncryptKey, CommsError>
{
    trace!("negotiating {}bit shared secret", key_size);

    // Generate the encryption keys
    let sk1 = crate::crypto::PrivateEncryptKey::generate(key_size);
    let pk1 = sk1.as_public_key();
    let pk1_bytes = pk1.pk();

    // Send our public key to the other side
    trace!("client sending its public key (and strength)");
    stream_tx.write_32bit(pk1_bytes, false).await?;

    // Receive one half of the secret that was just generated by the other side
    let iv1_bytes = stream_rx.read_32bit().await?;
    let iv1 = InitializationVector::from(iv1_bytes);
    let ek1 = match sk1.decapsulate(&iv1) {
        Some(a) => a,
        None => { bail!(CommsErrorKind::ReceiveError("Failed to decapsulate the encryption key from the received initialization vector.".to_string())); }
    };
    trace!("client received the servers half of the shared secret");

    // Receive the public key from the other side (which we will use in a sec)
    let pk2_bytes = stream_rx.read_32bit().await?;
    trace!("client received the servers public key");
    let pk2 = match PublicEncryptKey::from_bytes(pk2_bytes) {
        Some(a) => a,
        None => { bail!(CommsErrorKind::ReceiveError("Failed to receive a public key from the other side.".to_string())); }
    };

    // Validate the public key against our validation rules
    if validation.validate(&pk2.hash()) == false {
        bail!(CommsErrorKind::ServerCertificateValidation);
    }

    // Generate one half of the secret and send the IV so the other side can recreate it
    let (iv2, ek2) = pk2.encapsulate();
    stream_tx.write_32bit(&iv2.bytes[..], false).await?;
    trace!("client sending its half of the shared secret");
    
    // Merge the two halfs to make one shared secret
    trace!("client shared secret established");
    Ok(EncryptKey::xor(&ek1, &ek2))
}

#[cfg(feature = "enable_server")]
pub(super) async fn mesh_key_exchange_receiver(stream_rx: &mut StreamRx, stream_tx: &mut StreamTx, server_key: PrivateEncryptKey) -> Result<EncryptKey, CommsError>
{
    trace!("negotiating {}bit shared secret", server_key.size());

    // Receive the public key from the caller side (which we will use in a sec)
    let pk1_bytes = stream_rx.read_32bit().await?;
    trace!("server received clients public key");
    let pk1 = match PublicEncryptKey::from_bytes(pk1_bytes) {
        Some(a) => a,
        None => { bail!(CommsErrorKind::ReceiveError("Failed to receive a valid public key from the sender".to_string())); }
    };

    // Generate one half of the secret and send the IV so the other side can recreate it
    let (iv1, ek1) = pk1.encapsulate();
    trace!("server sending its half of the shared secret");
    stream_tx.write_32bit(&iv1.bytes[..], true).await?;

    let sk2 = server_key;
    let pk2 = sk2.as_public_key();
    let pk2_bytes = pk2.pk();

    // Send our public key to the other side
    trace!("server sending its public key");
    stream_tx.write_32bit(pk2_bytes, false).await?;

    // Receive one half of the secret that was just generated by the other side
    let iv2_bytes = stream_rx.read_32bit().await?;
    let iv2 = InitializationVector::from(iv2_bytes);
    let ek2 = match sk2.decapsulate(&iv2) {
        Some(a) => a,
        None => { bail!(CommsErrorKind::ReceiveError("Failed to receive a public key from the other side.".to_string())); }
    };
    trace!("server received client half of the shared secret");
    
    // Merge the two halfs to make one shared secret
    trace!("server shared secret established");
    Ok(EncryptKey::xor(&ek1, &ek2))
}
#![allow(unused_imports)]
use log::{info, warn, debug};
use tokio::io::{ AsyncReadExt, AsyncWriteExt};
use crate::crypto::{EncryptKey, PublicEncryptKey, InitializationVector};

use crate::error::*;
use crate::crypto::KeySize;

use super::StreamRx;
use super::StreamTx;

#[cfg(feature = "enable_client")]
pub(super) async fn mesh_key_exchange_sender(stream_rx: &mut StreamRx, stream_tx: &mut StreamTx, key_size: KeySize) -> Result<EncryptKey, CommsError>
{
    debug!("negotiating {}bit shared secret", key_size);

    // Generate the encryption keys
    let sk1 = crate::crypto::PrivateEncryptKey::generate(key_size);
    let pk1 = sk1.as_public_key();
    let pk1_bytes = pk1.pk();

    // Send our public key to the other side
    debug!("client sending its public key (and strength)");
    stream_tx.write_32bit(pk1_bytes, false).await?;

    // Receive one half of the secret that was just generated by the other side
    let iv1_bytes = stream_rx.read_32bit().await?;
    let iv1 = InitializationVector::from(iv1_bytes);
    let ek1 = match sk1.decapsulate(&iv1) {
        Some(a) => a,
        None => { return Err(CommsError::ReceiveError("Failed to decapsulate the encryption key from the received initialization vector.".to_string())); }
    };
    debug!("client received the servers half of the shared secret");

    // Receive the public key from the other side (which we will use in a sec)
    let pk2_bytes = stream_rx.read_32bit().await?;
    debug!("client received the servers public key");
    let pk2 = match PublicEncryptKey::from_bytes(pk2_bytes) {
        Some(a) => a,
        None => { return Err(CommsError::ReceiveError("Failed to receive a public key from the other side.".to_string())); }
    };

    // Generate one half of the secret and send the IV so the other side can recreate it
    let (iv2, ek2) = pk2.encapsulate();
    stream_tx.write_32bit(iv2.bytes, false).await?;
    debug!("client sending its half of the shared secret");
    
    // Merge the two halfs to make one shared secret
    debug!("client shared secret established");
    Ok(EncryptKey::xor(&ek1, &ek2))
}

#[cfg(all(feature = "enable_server", feature = "enable_tcp" ))]
pub(super) async fn mesh_key_exchange_receiver(stream_rx: &mut StreamRx, stream_tx: &mut StreamTx, key_size: KeySize) -> Result<EncryptKey, CommsError>
{
    debug!("negotiating {}bit shared secret", key_size);

    // Receive the public key from the caller side (which we will use in a sec)
    let pk1_bytes = stream_rx.read_32bit().await?;
    debug!("server received clients public key");
    let pk1 = match PublicEncryptKey::from_bytes(pk1_bytes) {
        Some(a) => a,
        None => { return Err(CommsError::ReceiveError("Failed to receive a valid public key from the sender".to_string())); }
    };

    // Generate one half of the secret and send the IV so the other side can recreate it
    let (iv1, ek1) = pk1.encapsulate();
    debug!("server sending its half of the shared secret");
    stream_tx.write_32bit(iv1.bytes, true).await?;

    let sk2 = crate::crypto::PrivateEncryptKey::generate(key_size);
    let pk2 = sk2.as_public_key();
    let pk2_bytes = pk2.pk();

    // Send our public key to the other side
    debug!("server sending its public key");
    stream_tx.write_32bit(pk2_bytes, false).await?;

    // Receive one half of the secret that was just generated by the other side
    let iv2_bytes = stream_rx.read_32bit().await?;
    let iv2 = InitializationVector::from(iv2_bytes);
    let ek2 = match sk2.decapsulate(&iv2) {
        Some(a) => a,
        None => { return Err(CommsError::ReceiveError("Failed to receive a public key from the other side.".to_string())); }
    };
    debug!("server received client half of the shared secret");
    
    // Merge the two halfs to make one shared secret
    debug!("server shared secret established");
    Ok(EncryptKey::xor(&ek1, &ek2))
}
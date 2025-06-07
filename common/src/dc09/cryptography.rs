use aes::cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit, block_padding::NoPadding};
use rand::{Rng, distr::Alphanumeric};

const ZEROS_IV: [u8; 16] = [0u8; 16];

/// Encrypts the DC09 message using AES CBC.\
/// **Note** that it adds padding if necessary.
pub fn encrypt(message: &str, key: &[u8]) -> Option<String> {
    match key.len() {
        16 => encrypt_internal::<cbc::Encryptor<aes::Aes128>>(message, key, &ZEROS_IV),
        24 => encrypt_internal::<cbc::Encryptor<aes::Aes192>>(message, key, &ZEROS_IV),
        32 => encrypt_internal::<cbc::Encryptor<aes::Aes256>>(message, key, &ZEROS_IV),
        _ => None,
    }
}

/// Decrypts the DC09 message using AES CBC.\
/// **Note** that it does not remove padding from the message.
pub fn decrypt(message: &str, key: &[u8]) -> Option<String> {
    let message = hex::decode(message).ok()?;
    match key.len() {
        16 => decrypt_internal::<cbc::Decryptor<aes::Aes128>>(message, key, &ZEROS_IV),
        24 => decrypt_internal::<cbc::Decryptor<aes::Aes192>>(message, key, &ZEROS_IV),
        32 => decrypt_internal::<cbc::Decryptor<aes::Aes256>>(message, key, &ZEROS_IV),
        _ => None,
    }
}

fn encrypt_internal<C>(message: &str, key: &[u8], iv: &[u8]) -> Option<String>
where
    C: BlockEncryptMut + KeyIvInit,
{
    let mut padded = pad_message(message).into_bytes();
    let message_len = padded.len();
    let cipher = C::new_from_slices(key, iv).ok()?;
    cipher.encrypt_padded_mut::<NoPadding>(&mut padded, message_len).ok()?;
    Some(hex::encode_upper(padded))
}

fn decrypt_internal<C>(mut message: Vec<u8>, key: &[u8], iv: &[u8]) -> Option<String>
where
    C: BlockDecryptMut + KeyIvInit,
{
    let cipher = C::new_from_slices(key, iv).ok()?;
    cipher.decrypt_padded_mut::<NoPadding>(&mut message).ok()?;
    Some(core::str::from_utf8(&message).ok()?.to_string())
}

/// Adds padding to the start of a DC09 message.
fn pad_message(message: &str) -> String {
    let pad = 16usize.saturating_sub(message.len() % 16usize);
    let mut rng = rand::rng();
    let mut result = String::with_capacity(message.len() + pad);
    for _ in 0..pad {
        result.push(rng.sample(Alphanumeric).into());
    }

    result.push_str(message);
    result
}

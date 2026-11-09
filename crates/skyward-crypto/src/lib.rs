pub mod fscrypt;
pub mod gatekeeper;
pub mod keymaster;
pub mod keys;
pub mod keystore;
pub mod locksettings;
pub mod sharedsecret;
pub mod token;
pub mod user;
pub mod weaver;

#[cfg(target_os = "android")]
pub mod data;
#[cfg(target_os = "android")]
pub mod keymint;
#[cfg(target_os = "android")]
pub mod keystore2;

#[cfg(target_os = "android")]
impl keystore::Decryptor for keymaster::Keymaster<'_> {
    fn gcm_decrypt(&self, key_blob: &[u8], _app_id: &[u8], nonce: &[u8], body: &[u8]) -> anyhow::Result<Vec<u8>> {
        // The service of the older kind has no notion of an application
        // the key is bound to; the tag of the key alone says what it may
        // be used for.
        let params = vec![
            keymaster::Parameter::number(keymaster::tag::BLOCK_MODE, keymaster::block::GCM),
            keymaster::Parameter::number(keymaster::tag::PADDING, keymaster::padding::NONE),
            keymaster::Parameter::bytes(keymaster::tag::NONCE, nonce.to_vec()),
            keymaster::Parameter::number(keymaster::tag::MAC_LENGTH, 128),
            keymaster::Parameter::number(keymaster::tag::MIN_MAC_LENGTH, 128),
        ];
        let (handle, _) = self.begin(keymaster::purpose::DECRYPT, key_blob, &params)?;
        match self.finish(handle, &[], body) {
            Ok(plain) => Ok(plain),
            Err(e) => {
                let _ = self.abort(handle);
                Err(e)
            }
        }
    }
}

#[cfg(target_os = "android")]
impl keystore::Decryptor for keymint::KeyMint<'_> {
    fn gcm_decrypt(&self, key_blob: &[u8], app_id: &[u8], nonce: &[u8], body: &[u8]) -> anyhow::Result<Vec<u8>> {
        // The keys unwrapped this way belong to the phone rather than to
        // any user, so none of them is guarded by a screen lock.
        keymint::KeyMint::gcm_decrypt(self, key_blob, app_id, nonce, body, None)
    }
}

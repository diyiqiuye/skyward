//! The phone's key service, of whichever kind it has. A phone whose own
//! system speaks the newer kind is served by that; one that speaks only
//! the older kind is served by that, which this reaches over the
//! hardware driver.

use anyhow::{Context, Result};
use skyward_binder::driver::Binder;
use std::path::Path;

use crate::keymaster::Keymaster;
use crate::keymint::KeyMint;
use crate::keystore::{StoredKey, Unwrap};

/// The phone's key service.
pub enum Keys<'a> {
    /// The newer kind, which the phone's own system speaks with.
    Newer(KeyMint<'a>),
    /// The older kind, which the phone's own system speaks with where it
    /// has no newer.
    Older(Keymaster<'a>),
}

impl<'a> Keys<'a> {
    /// The service of the phone: the newer kind where it has one, and the
    /// older kind where it has not.
    pub fn open(binder: &'a Binder, hardware: &'a Binder) -> Result<Keys<'a>> {
        match KeyMint::open(binder) {
            Ok(service) => Ok(Keys::Newer(service)),
            Err(e) => {
                let older = Keymaster::open(hardware)
                    .with_context(|| format!("the phone's key service could not be reached: {e:#}"))?;
                Ok(Keys::Older(older))
            }
        }
    }

    /// Which kind of service this is, for the words that say so.
    pub fn kind(&self) -> &'static str {
        match self {
            Keys::Newer(_) => "the newer kind",
            Keys::Older(_) => "the older kind",
        }
    }

    /// The key a directory holds, in the form the storage hardware takes
    /// for this start of the phone.
    ///
    /// Where the service is of the newer kind, a key the hardware holds
    /// never appears as itself: what comes back is wrapped again, for
    /// this start only. The service of the older kind has no such
    /// wrapping of its own, and gives the key as it is.
    pub fn hardware_key(&self, dir: &Path, wrapped: bool) -> Result<Vec<u8>> {
        let stored = StoredKey::load(dir).with_context(|| format!("read the key in {}", dir.display()))?;
        let key = match self {
            Keys::Newer(service) => stored
                .unwrap(Unwrap::Hardware(service))
                .with_context(|| format!("unwrap the key in {}", dir.display()))?,
            Keys::Older(service) => {
                // The blob may stand for the key itself, which the secure
                // world gives out whole, or for a key that wraps it, which
                // it opens.
                let exported = match stored.blob() {
                    Some(blob) => service.export_key(blob).ok(),
                    None => None,
                };
                match exported {
                    Some(key) => key,
                    None => stored
                        .unwrap(Unwrap::Hardware(service))
                        .with_context(|| format!("unwrap the key in {}", dir.display()))?,
                }
            }
        };
        match (wrapped, self) {
            (true, Keys::Newer(service)) => service
                .ephemeral_storage_key(&key)
                .with_context(|| format!("make the key in {} usable now", dir.display())),
            _ => Ok(key),
        }
    }
}

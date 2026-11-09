//! The phone's key service of the older kind, the one that talks to the
//! secure world over the hardware driver. A phone whose system speaks the
//! newer one is served by that instead; this is what a phone whose
//! vendor carries only the older one has.
//!
//! The service numbers its methods in the order the interface declares
//! them, and this follows that order: the nineteenth of them is the last.

use anyhow::{bail, Context, Result};
use skyward_binder::driver::{Binder, Handle};
use skyward_binder::hidl::{Answer, Message};

/// The service, as the phone's own manifest names it.
pub const SERVICE: &str = "android.hardware.keymaster@4.0::IKeymasterDevice";
const INSTANCE: &str = "default";

/// `generateKey`, sixth of the nineteen.
pub const GENERATE_KEY: u32 = 6;
/// `getKeyCharacteristics`, ninth.
pub const GET_KEY_CHARACTERISTICS: u32 = 9;
/// `exportKey`, tenth.
pub const EXPORT_KEY: u32 = 10;
/// `begin`, sixteenth.
pub const BEGIN: u32 = 16;
/// `update`, seventeenth.
pub const UPDATE: u32 = 17;
/// `finish`, eighteenth.
pub const FINISH: u32 = 18;
/// `abort`, nineteenth.
pub const ABORT: u32 = 19;

/// The tags the interface gives the parameters of a call.
pub mod tag {
    pub const PURPOSE: u32 = 0x2000_0001;
    pub const ALGORITHM: u32 = 0x1000_0002;
    pub const KEY_SIZE: u32 = 0x3000_0003;
    pub const BLOCK_MODE: u32 = 0x2000_0004;
    pub const DIGEST: u32 = 0x2000_0005;
    pub const PADDING: u32 = 0x2000_0006;
    pub const NONCE: u32 = 0x9000_03e9;
    pub const MAC_LENGTH: u32 = 0x3000_03eb;
    pub const MIN_MAC_LENGTH: u32 = 0x3000_0008;
    pub const NO_AUTH_REQUIRED: u32 = 0x7000_01f7;
    pub const CALLER_NONCE: u32 = 0x7000_0007;
    /// What a key is bound to, for the service of the older kind, which
    /// has no other way of being told.
    pub const ASSOCIATED_DATA: u32 = 0x7000_03e8;
    /// What a key is bound to: the directory the key belongs to.
    pub const APPLICATION_ID: u32 = 0x7000_0259;
}

/// The purposes a key may be used for.
pub mod purpose {
    pub const ENCRYPT: u32 = 0;
    pub const DECRYPT: u32 = 1;
}

/// The algorithms a key may be used with.
pub mod algorithm {
    pub const AES: u64 = 32;
}

/// The modes a cipher may run in.
pub mod block {
    pub const GCM: u64 = 32;
}

/// The paddings a cipher may use.
pub mod padding {
    pub const NONE: u64 = 1;
}

/// One parameter of a call: what it says, the number it says, and the bytes
/// it says when what it says is bytes.
#[derive(Clone, Debug)]
pub struct Parameter {
    pub tag: u32,
    pub value: u64,
    pub blob: Vec<u8>,
}

impl Parameter {
    /// A parameter that says a number.
    pub fn number(tag: u32, value: u64) -> Parameter {
        Parameter { tag, value, blob: Vec::new() }
    }

    /// A parameter that says bytes.
    pub fn bytes(tag: u32, blob: Vec<u8>) -> Parameter {
        Parameter { tag, value: 0, blob }
    }
}

/// The bytes one parameter is written as: the tag, the number, and where
/// the bytes are and how many of them there are.
const ELEMENT: usize = 32;

/// Where inside an element the address of its own bytes is kept.
const BLOB_AT: u64 = 0x10;

/// The token that proves a user was asked for, as the interface lays it
/// out: seven numbers of eight bytes each.
const TOKEN: usize = 56;

/// The token that proves a user was asked for, as the interface lays it
/// out: seven numbers of eight bytes each, and the place inside it where
/// the bytes of its own are kept.
fn write_auth_token(call: &mut Message) {
    let index = call.raw(&[0u8; TOKEN]);
    call.inside(index, 0x28, &[]);
}

/// The token that says what another service has verified: a challenge, a
/// time, the parameters that were verified and the bytes of the
/// verification's own, each of them kept at its place inside the token.
fn write_verification_token(call: &mut Message) {
    let index = call.raw(&[0u8; TOKEN]);
    call.inside(index, 0x10, &[]);
    call.inside(index, 0x28, &[]);
}

/// The phone's key service.
pub struct Keymaster<'a> {
    device: Handle<'a>,
}

impl<'a> Keymaster<'a> {
    /// Reach the running key service over the hardware driver.
    pub fn open(hardware: &'a Binder) -> Result<Keymaster<'a>> {
        let device = skyward_binder::hidl::find(hardware, SERVICE, INSTANCE)
            .context("look the key service up")?
            .context("the key service is not running; the phone's own services have to be started first")?;
        Ok(Keymaster { device })
    }

    /// Write the parameters of a call: the run of them, then the bytes
    /// each of them carries beside itself.
    fn write_parameters(call: &mut Message, params: &[Parameter]) {
        // The bytes of each parameter are held first, so that the element
        // which points at them can be written with the address where they
        // are.
        let mut held: Vec<(u64, usize)> = Vec::with_capacity(params.len());
        for p in params {
            let at = call.hold(p.blob.clone());
            held.push((at, p.blob.len()));
        }
        let mut body = Vec::with_capacity(params.len() * ELEMENT);
        for (i, p) in params.iter().enumerate() {
            let mut e = [0u8; ELEMENT];
            e[0..4].copy_from_slice(&p.tag.to_le_bytes());
            e[8..16].copy_from_slice(&p.value.to_le_bytes());
            e[BLOB_AT as usize..BLOB_AT as usize + 8].copy_from_slice(&held[i].0.to_le_bytes());
            e[24..28].copy_from_slice(&(held[i].1 as u32).to_le_bytes());
            body.extend_from_slice(&e);
        }
        let (_frame, body_index) = call.elements(params.len() as u32, &body);
        for (i, _) in params.iter().enumerate() {
            // Every element keeps the address of its own bytes, and the far
            // side reads one for each of them.
            call.element_bytes(body_index, (i * ELEMENT) as u64 + BLOB_AT, held[i].0, held[i].1);
        }
    }

    /// The error the service gave, as its own numbering.
    fn code(read: &mut Answer) -> Result<i32> {
        read.i32().context("read the error the service gave")
    }

    /// A run of parameters the service answered with: the header that says
    /// how many there are, the bytes of all of them together, and then the
    /// bytes each of them carries beside itself.
    fn answered_parameters(read: &mut Answer) -> Result<Vec<Parameter>> {
        let header = read.answered().context("read the header of what the service answered")?;
        let count = if header.len() >= 16 {
            u64::from_le_bytes(header[8..16].try_into().expect("eight bytes")) as usize
        } else {
            0
        };
        let body = if read.next_is_object() {
            read.answered().context("read what the service answered")?.to_vec()
        } else {
            Vec::new()
        };
        for _ in 0..count {
            if !read.next_is_object() {
                break;
            }
            let _ = read.answered().context("read what the service answered beside the parameters")?;
        }
        Ok(parameters_of(&body))
    }

    /// A run of bytes the service answered with: the header that says how
    /// many there are, then the bytes themselves.
    fn answered(read: &mut Answer) -> Result<Vec<u8>> {
        let _header = read.answered().context("read the header of what the service answered")?;
        if !read.next_is_object() {
            return Ok(Vec::new());
        }
        let body = read.answered().context("read what the service answered")?;
        Ok(body.to_vec())
    }

    /// Make a key of the given algorithm, for the given purposes, and give
    /// back the blob that stands for it.
    pub fn generate_key(&self, params: &[Parameter]) -> Result<Vec<u8>> {
        let mut call = Message::call(SERVICE);
        Self::write_parameters(&mut call, params);
        let reply = self.device.transact(GENERATE_KEY, &call).context("have the secure world make a key")?;
        let mut read = Answer::new(reply.bytes());
        read.status().context("have the secure world make a key")?;
        let code = Self::code(&mut read)?;
        if code != 0 {
            bail!("the secure world refused to make the key: {code}");
        }
        Self::answered(&mut read)
    }

    /// What the service says about a key: the parameters that hold for it.
    pub fn characteristics(&self, blob: &[u8]) -> Result<Vec<Parameter>> {
        self.characteristics_bound(blob, &[], &[])
    }

    /// What the service says about a key, asked with who is asking and what
    /// the key is bound to. A key that was made for a caller is only
    /// described to that caller, and the secure world turns away a
    /// question about a key bound to another.
    pub fn characteristics_bound(&self, blob: &[u8], client_id: &[u8], app_data: &[u8]) -> Result<Vec<Parameter>> {
        let mut call = Message::call(SERVICE);
        call.bytes(blob);
        call.bytes(client_id);
        call.bytes(app_data);
        let reply = self.device.transact(GET_KEY_CHARACTERISTICS, &call).context("ask about the key")?;
        let mut read = Answer::new(reply.bytes());
        read.status().context("ask about the key")?;
        let code = Self::code(&mut read)?;
        if code != 0 {
            bail!("the secure world does not know that key: {code}");
        }
        // The characteristics are a structure of two runs of parameters,
        // each with a header of its own.
        let _header = read.answered().context("read what the secure world said about the key")?;
        let mut out = Vec::new();
        for _ in 0..2 {
            if !read.next_is_object() {
                break;
            }
            let raw = read.answered().context("read what the secure world said about the key")?;
            out.extend(parameters_of(raw));
        }
        Ok(out)
    }

    /// The key the blob stands for, as the secure world gives it out: a
    /// key that is not exportable is refused.
    pub fn export_key(&self, blob: &[u8]) -> Result<Vec<u8>> {
        let mut call = Message::call(SERVICE);
        call.u32(1);
        call.bytes(blob);
        call.bytes(&[]);
        call.bytes(&[]);
        let reply = self.device.transact(EXPORT_KEY, &call).context("ask for the key")?;
        let mut read = Answer::new(reply.bytes());
        read.status().context("ask for the key")?;
        let code = Self::code(&mut read)?;
        if code != 0 {
            bail!("the secure world will not give out that key: {code}");
        }
        Self::answered(&mut read)
    }

    /// Begin an operation on a key: the handle it is known by, and the
    /// parameters the secure world answers with.
    pub fn begin(&self, purpose: u32, blob: &[u8], params: &[Parameter]) -> Result<(u64, Vec<Parameter>)> {
        let mut call = Message::call(SERVICE);
        call.u32(purpose);
        call.bytes(blob);
        Self::write_parameters(&mut call, params);
        write_auth_token(&mut call);
        let reply = self.device.transact(BEGIN, &call).context("begin the operation")?;
        if std::env::var("SKYWARD_DEBUG").is_ok() {
            eprintln!("begin reply: {:02x?}", reply.bytes());
        }
        let mut read = Answer::new(reply.bytes());
        read.status().context("begin the operation")?;
        let code = Self::code(&mut read)?;
        if code != 0 {
            bail!("the secure world refused to begin: {code}");
        }
        let out = Self::answered_parameters(&mut read)?;
        let handle = read.i64().context("read the handle of the operation")? as u64;
        Ok((handle, out))
    }

    /// Give the operation more of the message, and take what it answers.
    pub fn update(&self, handle: u64, params: &[Parameter], input: &[u8]) -> Result<Vec<u8>> {
        let mut call = Message::call(SERVICE);
        call.u64(handle);
        Self::write_parameters(&mut call, params);
        call.bytes(input);
        call.bytes(&[]);
        write_auth_token(&mut call);
        write_verification_token(&mut call);
        let reply = self.device.transact(UPDATE, &call).context("carry the operation on")?;
        let mut read = Answer::new(reply.bytes());
        read.status().context("carry the operation on")?;
        let code = Self::code(&mut read)?;
        if code != 0 {
            bail!("the secure world refused to carry the operation on: {code}");
        }
        let _consumed = read.i32().context("read how much the secure world took")?;
        let _out = Self::answered_parameters(&mut read)?;
        Self::answered(&mut read)
    }

    /// Finish the operation and take what it answers: the message, read.
    pub fn finish(&self, handle: u64, params: &[Parameter], input: &[u8]) -> Result<Vec<u8>> {
        let mut call = Message::call(SERVICE);
        call.u64(handle);
        Self::write_parameters(&mut call, params);
        call.bytes(input);
        call.bytes(&[]);
        write_auth_token(&mut call);
        write_verification_token(&mut call);
        let reply = self.device.transact(FINISH, &call).context("finish the operation")?;
        let mut read = Answer::new(reply.bytes());
        read.status().context("finish the operation")?;
        let code = Self::code(&mut read)?;
        if code != 0 {
            bail!("the secure world refused to finish: {code}");
        }
        let _out = Self::answered_parameters(&mut read)?;
        Self::answered(&mut read)
    }

    /// Give up on the operation.
    pub fn abort(&self, handle: u64) -> Result<()> {
        let mut call = Message::call(SERVICE);
        call.u64(handle);
        let reply = self.device.transact(ABORT, &call).context("give up the operation")?;
        let mut read = Answer::new(reply.bytes());
        read.status().context("give up the operation")?;
        let code = Self::code(&mut read)?;
        if code != 0 {
            bail!("the secure world refused to give the operation up: {code}");
        }
        Ok(())
    }
}

/// The parameters a call answers with, as they are written.
fn parameters_of(raw: &[u8]) -> Vec<Parameter> {
    raw.chunks(ELEMENT)
        .map(|e| {
            let tag = u32::from_le_bytes(e[0..4].try_into().expect("four bytes"));
            let value = u64::from_le_bytes(e[8..16].try_into().expect("eight bytes"));
            Parameter { tag, value, blob: Vec::new() }
        })
        .collect()
}

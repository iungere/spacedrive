use crate::{
	primitives::{
		AES_256_GCM_NONCE_LEN, AES_256_GCM_SIV_NONCE_LEN, ENCRYPTED_KEY_LEN, SALT_LEN,
		XCHACHA20_POLY1305_NONCE_LEN,
	},
	types::{Algorithm, EncryptedKey, HashingAlgorithm, Nonce, Params, Salt},
	utils::ToArray,
	Error, Result,
};

use self::{
	header::HeaderVersion,
	keyslot::Keyslot,
	object::{HeaderObject, HeaderObjectIdentifier},
};

use super::Header;

pub mod header;
pub mod keyslot;
pub mod object;

pub struct Offset(usize);

impl Offset {
	#[must_use]
	pub const fn new(v: usize) -> Self {
		Self(v)
	}

	#[must_use]
	pub fn increment(&mut self, v: usize) -> usize {
		self.0 += v;
		self.0
	}
}

const KEYSLOT_LIMIT: usize = 2;
const OBJECT_LIMIT: usize = 2;

pub trait HeaderEncode {
	const OUTPUT_LEN: usize;
	type Output;

	fn as_bytes(&self) -> Self::Output;
	fn from_bytes(b: Self::Output) -> Result<Self>
	where
		Self: Sized;
}

impl HeaderEncode for Params {
	const OUTPUT_LEN: usize = 1;
	type Output = u8;

	fn as_bytes(&self) -> Self::Output {
		match self {
			Self::Standard => 18u8,
			Self::Hardened => 39u8,
			Self::Paranoid => 56u8,
		}
	}

	fn from_bytes(b: Self::Output) -> Result<Self> {
		match b {
			18u8 => Ok(Self::Standard),
			39u8 => Ok(Self::Hardened),
			56u8 => Ok(Self::Paranoid),
			_ => Err(Error::Validity),
		}
	}
}

impl HeaderEncode for HashingAlgorithm {
	const OUTPUT_LEN: usize = 1 + Params::OUTPUT_LEN;
	type Output = [u8; Self::OUTPUT_LEN];

	fn as_bytes(&self) -> Self::Output {
		match self {
			Self::Argon2id(p) => [0xF2u8, p.as_bytes()],
			Self::Blake3Balloon(p) => [0xA8u8, p.as_bytes()],
		}
	}

	fn from_bytes(b: Self::Output) -> Result<Self> {
		let x = match b[0] {
			0xF2u8 => Self::Argon2id(Params::from_bytes(b[1])?),
			0xA8u8 => Self::Blake3Balloon(Params::from_bytes(b[1])?),
			_ => return Err(Error::Validity),
		};

		Ok(x)
	}
}

impl HeaderEncode for Algorithm {
	const OUTPUT_LEN: usize = 2;
	type Output = [u8; Self::OUTPUT_LEN];

	fn as_bytes(&self) -> Self::Output {
		let s = match self {
			Self::Aes256Gcm => 0xD1,
			Self::Aes256GcmSiv => 0xD3,
			Self::XChaCha20Poly1305 => 0xD5,
		};

		[13u8, s]
	}

	fn from_bytes(b: Self::Output) -> Result<Self> {
		if b[0] != 13u8 {
			return Err(Error::Validity);
		}

		let a = match b[1] {
			0xD1 => Self::Aes256Gcm,
			0xD3 => Self::Aes256GcmSiv,
			0xD5 => Self::XChaCha20Poly1305,
			_ => return Err(Error::Validity),
		};

		Ok(a)
	}
}

impl HeaderEncode for Salt {
	const OUTPUT_LEN: usize = SALT_LEN + 2;
	type Output = [u8; Self::OUTPUT_LEN];

	fn as_bytes(&self) -> Self::Output {
		let mut s = [0u8; Self::OUTPUT_LEN];
		s[0] = 12u8;
		s[1] = 4u8;
		s[2..].copy_from_slice(self.inner());
		s
	}

	fn from_bytes(b: Self::Output) -> Result<Self> {
		if b[..2] != [12u8, 4u8] {
			return Err(Error::Validity);
		}

		let mut o = [0u8; 16];
		o.copy_from_slice(&b[2..]);

		Ok(Self::new(o))
	}
}

impl HeaderEncode for Nonce {
	const OUTPUT_LEN: usize = 32;
	type Output = [u8; Self::OUTPUT_LEN];

	fn as_bytes(&self) -> Self::Output {
		let b = match self {
			Self::Aes256Gcm(_) => 0xB2u8,
			Self::Aes256GcmSiv(_) => 0xB5u8,
			Self::XChaCha20Poly1305(_) => 0xB7u8,
		};

		let len = self.algorithm().nonce_len();

		let mut s = [0u8; Self::OUTPUT_LEN];
		s[0] = 99u8;
		s[1] = b;
		s[2..len + 2].copy_from_slice(self.inner());

		s[len + 2..].copy_from_slice(&self.inner()[..Self::OUTPUT_LEN - 2 - len]);

		s
	}

	fn from_bytes(b: Self::Output) -> Result<Self> {
		if b[0] != 99u8 {
			return Err(Error::Validity);
		}

		let x = match b[1] {
			0xB2u8 => Self::Aes256Gcm(b[2..2 + AES_256_GCM_NONCE_LEN].to_array()?),
			0xB5u8 => Self::Aes256GcmSiv(b[2..2 + AES_256_GCM_SIV_NONCE_LEN].to_array()?),
			0xB7u8 => Self::XChaCha20Poly1305(b[2..2 + XCHACHA20_POLY1305_NONCE_LEN].to_array()?),
			_ => return Err(Error::Validity),
		};

		Ok(x)
	}
}

impl HeaderEncode for EncryptedKey {
	const OUTPUT_LEN: usize = ENCRYPTED_KEY_LEN + Nonce::OUTPUT_LEN + 2;
	type Output = [u8; Self::OUTPUT_LEN];

	fn as_bytes(&self) -> Self::Output {
		let mut s = [0u8; Self::OUTPUT_LEN];
		s[0] = 9u8;
		s[1] = 0xF3u8;

		let mut offset = Offset::new(2);

		s[offset.0..offset.increment(ENCRYPTED_KEY_LEN)].copy_from_slice(self.inner());
		s[offset.0..].copy_from_slice(&self.nonce().as_bytes());
		s
	}

	fn from_bytes(b: Self::Output) -> Result<Self> {
		if b[..2] != [9u8, 0xF3u8] {
			return Err(Error::Validity);
		}

		let mut offset = Offset::new(2);

		let mut e = [0u8; ENCRYPTED_KEY_LEN];

		e.copy_from_slice(&b[offset.0..offset.increment(ENCRYPTED_KEY_LEN)]);
		let n = Nonce::from_bytes(b[offset.0..].to_array()?)?;

		Ok(Self::new(e, n))
	}
}

impl HeaderEncode for Keyslot {
	const OUTPUT_LEN: usize =
		EncryptedKey::OUTPUT_LEN + (Salt::OUTPUT_LEN * 2) + HashingAlgorithm::OUTPUT_LEN + 2;
	type Output = [u8; Self::OUTPUT_LEN];

	fn as_bytes(&self) -> Self::Output {
		let mut o = [0u8; Self::OUTPUT_LEN];
		o[0] = 0x83;
		o[1] = 0x21;

		let mut offset = Offset::new(2);

		o[offset.0..offset.increment(HashingAlgorithm::OUTPUT_LEN)]
			.copy_from_slice(&self.hashing_algorithm.as_bytes());
		o[offset.0..offset.increment(Salt::OUTPUT_LEN)].copy_from_slice(&self.hash_salt.as_bytes());
		o[offset.0..offset.increment(Salt::OUTPUT_LEN)].copy_from_slice(&self.salt.as_bytes());
		o[offset.0..].copy_from_slice(&self.encrypted_key.as_bytes());

		o
	}

	fn from_bytes(b: Self::Output) -> Result<Self> {
		if b[..2] != [0x83, 0x21] {
			return Err(Error::Validity);
		}

		let mut offset = Offset::new(2);
		let hashing_algorithm =
			HashingAlgorithm::from_bytes(b[offset.0..offset.increment(2)].to_array()?)?;
		let hash_salt =
			Salt::from_bytes(b[offset.0..offset.increment(Salt::OUTPUT_LEN)].to_array()?)?;
		let salt = Salt::from_bytes(b[offset.0..offset.increment(Salt::OUTPUT_LEN)].to_array()?)?;
		let ek = EncryptedKey::from_bytes(b[offset.0..].to_array()?)?;

		Ok(Self {
			hashing_algorithm,
			hash_salt,
			salt,
			encrypted_key: ek,
		})
	}
}

impl HeaderEncode for HeaderObject {
	const OUTPUT_LEN: usize = 0;
	type Output = Vec<u8>;

	fn as_bytes(&self) -> Self::Output {
		let mut o = vec![];

		o.extend_from_slice(&[0xF1, 51u8]);
		o.extend_from_slice(&self.identifier.as_bytes());
		o.extend_from_slice(&self.nonce.as_bytes());
		o.extend_from_slice(&(self.data.len() as u64).to_le_bytes());
		o.extend_from_slice(&self.data);

		o
	}

	fn from_bytes(b: Self::Output) -> Result<Self> {
		if b[..2] != [0xF1, 51u8] {
			return Err(Error::Validity);
		}

		let mut offset = Offset::new(2);
		let identifier = HeaderObjectIdentifier::from_bytes(
			b[offset.0..(offset.increment(HeaderObjectIdentifier::OUTPUT_LEN))].to_array()?,
		)?;
		let nonce =
			Nonce::from_bytes(b[offset.0..(offset.increment(Nonce::OUTPUT_LEN))].to_array()?)?;
		let data_len = u64::from_le_bytes(b[offset.0..offset.increment(8)].to_array()?);
		#[allow(clippy::cast_possible_truncation)]
		let data = b[offset.0..offset.increment(data_len as usize)].to_vec();

		Ok(Self {
			identifier,
			nonce,
			data,
		})
	}
}

impl HeaderEncode for HeaderObjectIdentifier {
	const OUTPUT_LEN: usize = 2 + EncryptedKey::OUTPUT_LEN + Salt::OUTPUT_LEN;
	type Output = [u8; Self::OUTPUT_LEN];

	fn as_bytes(&self) -> Self::Output {
		let mut o = [0u8; Self::OUTPUT_LEN];
		o[0] = 0xC2;
		o[1] = 0xE9;

		let mut offset = Offset::new(2);

		o[offset.0..offset.increment(EncryptedKey::OUTPUT_LEN)]
			.copy_from_slice(&self.key.as_bytes());
		o[offset.0..].copy_from_slice(&self.salt.as_bytes());

		o
	}

	fn from_bytes(b: Self::Output) -> Result<Self> {
		if b[..2] != [0xC2, 0xE9] {
			return Err(Error::Validity);
		}

		let mut offset = Offset::new(2);
		let ek = EncryptedKey::from_bytes(
			b[offset.0..offset.increment(EncryptedKey::OUTPUT_LEN)].to_array()?,
		)?;
		let salt = Salt::from_bytes(b[offset.0..].to_array()?)?;

		Ok(Self { key: ek, salt })
	}
}

impl HeaderEncode for HeaderVersion {
	const OUTPUT_LEN: usize = 2;
	type Output = [u8; Self::OUTPUT_LEN];

	fn as_bytes(&self) -> Self::Output {
		match self {
			Self::V1 => [0xDA; 2],
		}
	}

	fn from_bytes(b: Self::Output) -> Result<Self> {
		match b {
			[0xDA, 0xDA] => Ok(Self::V1),
			_ => Err(Error::Validity),
		}
	}
}

impl Header {
	#[must_use]
	pub fn as_bytes(&self) -> Vec<u8> {
		let mut o = vec![];
		o.extend_from_slice(&[0xFA, 0xDA]);

		o.extend_from_slice(&self.version.as_bytes());
		o.extend_from_slice(&self.algorithm.as_bytes());
		o.extend_from_slice(&self.nonce.as_bytes());

		self.keyslots
			.iter()
			.for_each(|k| o.extend_from_slice(&k.as_bytes()));

		(0..KEYSLOT_LIMIT - self.keyslots.len())
			.for_each(|_| o.extend_from_slice(&Keyslot::random().as_bytes()));

		#[allow(clippy::cast_possible_truncation)]
		o.extend_from_slice(&(self.objects.len() as u16).to_le_bytes());

		self.objects.iter().for_each(|k| {
			let b = k.as_bytes();
			o.extend_from_slice(&(b.len() as u64).to_le_bytes());
			o.extend_from_slice(&b);
		});

		o
	}

	pub(super) fn from_reader_raw<R>(reader: &mut R) -> Result<Self>
	where
		R: std::io::Read + std::io::Seek,
	{
		let mut m = [0u8; 2];
		reader.read_exact(&mut m)?;

		if m != [0xFA, 0xDA] {
			return Err(Error::Validity);
		}

		let mut buffer = [0u8; HeaderVersion::OUTPUT_LEN];
		reader.read_exact(&mut buffer)?;
		let version = HeaderVersion::from_bytes(buffer)?;

		let mut buffer = [0u8; Algorithm::OUTPUT_LEN];
		reader.read_exact(&mut buffer)?;
		let algorithm = Algorithm::from_bytes(buffer)?;

		let mut nonce_buffer = [0u8; Nonce::OUTPUT_LEN];
		reader.read_exact(&mut nonce_buffer)?;
		let nonce = Nonce::from_bytes(nonce_buffer)?;
		nonce.validate(algorithm)?;

		let keyslots = (0..KEYSLOT_LIMIT)
			.filter_map(|_| {
				let mut buffer = [0u8; Keyslot::OUTPUT_LEN];
				reader.read_exact(&mut buffer).ok();
				Keyslot::from_bytes(buffer).ok()
			})
			.collect::<Vec<Keyslot>>();

		let mut buffer = [0u8; 2];
		reader.read_exact(&mut buffer)?;
		let objects_len = u16::from_le_bytes(buffer);

		let objects = (0..objects_len as usize)
			.map(|_| {
				let mut buffer = [0u8; 8];
				reader.read_exact(&mut buffer)?;
				let size = u64::from_le_bytes(buffer);

				#[allow(clippy::cast_possible_truncation)]
				let mut buffer = vec![0u8; size as usize];
				reader.read_exact(&mut buffer)?;

				HeaderObject::from_bytes(buffer)
			})
			.collect::<Result<Vec<HeaderObject>>>()?;

		let h = Self {
			version,
			algorithm,
			nonce,
			keyslots,
			objects,
		};

		Ok(h)
	}
}
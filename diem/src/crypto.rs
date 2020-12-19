use crate::error::DiemError;
use ed25519_dalek as dalek;
use ed25519_dalek::ed25519;
use ed25519_dalek::Signer as _;
use rand::rngs::OsRng;
use rand::{CryptoRng, Rng};
use serde::{de, ser, Deserialize, Serialize};
use std::convert::TryInto;
use std::fmt;

#[cfg(test)]
#[path = "tests/crypto_tests.rs"]
pub mod crypto_tests;

pub type Digest = [u8; 32];

pub trait Digestible {
    fn digest(&self) -> Digest;
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct PublicKey(pub [u8; 32]);

impl PublicKey {
    pub fn to_base64(&self) -> String {
        base64::encode(&self.0[..])
    }

    pub fn from_base64(s: &str) -> Result<Self, base64::DecodeError> {
        let bytes = base64::decode(s)?;
        Ok(Self(bytes[..32].try_into().unwrap()))
    }
}

impl fmt::Debug for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self.to_base64())
    }
}

impl Serialize for PublicKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_str(&self.to_base64())
    }
}

impl<'de> Deserialize<'de> for PublicKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let value = Self::from_base64(&s).map_err(|e| de::Error::custom(e.to_string()))?;
        Ok(value)
    }
}

pub struct SecretKey([u8; 64]);

impl SecretKey {
    pub fn from_base64(s: &str) -> Result<Self, base64::DecodeError> {
        let bytes = base64::decode(s)?;
        Ok(Self(bytes[..64].try_into().unwrap()))
    }

    pub fn encode_base64(&self) -> String {
        base64::encode(&self.0[..])
    }
}

impl Drop for SecretKey {
    fn drop(&mut self) {
        self.0.iter_mut().for_each(|x| *x = 0);
    }
}

pub fn generate_production_keypair() -> (PublicKey, SecretKey) {
    generate_keypair(&mut OsRng)
}

pub fn generate_keypair<R>(csprng: &mut R) -> (PublicKey, SecretKey)
where
    R: CryptoRng + Rng,
{
    let keypair = dalek::Keypair::generate(csprng);
    let public = PublicKey(keypair.public.to_bytes());
    let secret = SecretKey(keypair.to_bytes());
    (public, secret)
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Signature {
    part1: [u8; 32],
    part2: [u8; 32],
}

impl Signature {
    pub fn new<D: Digestible>(data: &D, secret: &SecretKey) -> Self {
        let keypair = dalek::Keypair::from_bytes(&secret.0).unwrap();
        let digest = data.digest();
        let signature = keypair.sign(&digest).to_bytes();
        let part1 = signature[..32].try_into().unwrap();
        let part2 = signature[32..64].try_into().unwrap();
        Signature { part1, part2 }
    }

    fn flatten(&self) -> [u8; 64] {
        [self.part1, self.part2].concat().try_into().unwrap()
    }

    pub fn verify<D: Digestible>(&self, data: &D, public_key: &PublicKey) -> Result<(), DiemError> {
        let signature = ed25519::signature::Signature::from_bytes(&self.flatten())?;
        let public_key = dalek::PublicKey::from_bytes(&public_key.0)?;
        let digest = data.digest();
        public_key.verify_strict(&digest, &signature)?;
        Ok(())
    }

    pub fn verify_batch<'a, I, D>(data: &'a D, votes: I) -> Result<(), DiemError>
    where
        I: IntoIterator<Item = &'a (PublicKey, Signature)>,
        D: Digestible,
    {
        let digest = data.digest();
        let mut messages: Vec<&[u8]> = Vec::new();
        let mut signatures: Vec<dalek::Signature> = Vec::new();
        let mut public_keys: Vec<dalek::PublicKey> = Vec::new();
        for (key, sig) in votes.into_iter() {
            messages.push(&digest[..]);
            signatures.push(ed25519::signature::Signature::from_bytes(&sig.flatten())?);
            public_keys.push(dalek::PublicKey::from_bytes(&key.0)?);
        }
        dalek::verify_batch(&messages[..], &signatures[..], &public_keys[..])?;
        Ok(())
    }
}
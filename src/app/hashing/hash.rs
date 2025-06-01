use std::fs;
use anyhow::Context;
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, RECOMMENDED_SALT_LEN};
use rand::rand_core::OsRng;
use rand::TryRngCore;
use sha3::digest::DynDigest;
use sha3::Digest;

enum FileDriver {
    Sha3_224,
    Sha3_256,
    Sha3_384,
    Sha3_512,
}

enum PasswordDriver {
    Bcrypt,
    Argon2i,
    Argon2id,
}

impl PasswordDriver {
    fn from_env() -> Self {
        match std::env::var("HASH_DRIVER").as_deref() {
            Ok("bcrypt") => Self::Bcrypt,
            Ok("argon2i") => Self::Argon2i,
            Ok("argon2id") => Self::Argon2id,
            _ => Self::Bcrypt,
        }
    }
}

impl FileDriver {
    fn from_env() -> Self {
        match std::env::var("FILE_DRIVER").as_deref() {
            Ok("sha3_224") => Self::Sha3_224,
            Ok("sha3_256") => Self::Sha3_256,
            Ok("sha3_384") => Self::Sha3_384,
            Ok("sha3_512") => Self::Sha3_512,
            _ => Self::Sha3_256,
        }
    }
}

pub fn hash_password(password: &str) -> anyhow::Result<String> {
    match PasswordDriver::from_env() {
        PasswordDriver::Bcrypt => {
            let cost = std::env::var("BRCYPT_COST")
                .unwrap_or_else(|_| "12".into())
                .parse()?;

            Ok(bcrypt::hash(password, cost)?)
        }
        PasswordDriver::Argon2i | PasswordDriver::Argon2id => {
            let mem: u32 = std::env::var("ARGON2_MEMORY")
                .unwrap_or_else(|_| "4096".into())
                .parse()?;

            let time: u32 = std::env::var("ARGON2_TIME")
                .unwrap_or_else(|_| "3".into())
                .parse()?;
            let thread: u32 = std::env::var("ARGON2_THREADS")
                .unwrap_or_else(|_| "1".into())
                .parse()?;

            let params =
                argon2::Params::new(mem, time, thread, None).expect("Invalid Argon2 parameters");
            let mut random_salt = OsRng;

            let mut bytes = [0u8; RECOMMENDED_SALT_LEN];
            random_salt.try_fill_bytes(&mut bytes)?;
            let hasher = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);
            let salt = SaltString::encode_b64(&mut bytes)?;
            let hash = hasher
                .hash_password(password.as_bytes(), &salt)?
                .to_string();
            Ok(hash)
        }
    }
}

pub fn verify_password(hashed_password: &str, password: &str) -> anyhow::Result<bool> {
    let verified = if hashed_password.starts_with("$2b$") {
        bcrypt::verify(password, hashed_password).context("Failed")?
    } else {
        let parsed = PasswordHash::new(&hashed_password)?;
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok()
    };
    Ok(verified)
}

pub fn hash_file(data: &[u8]) -> anyhow::Result<String> {
    let mut hasher: Box<dyn DynDigest> = match FileDriver::from_env() {
        FileDriver::Sha3_224 => Box::new(sha3::Sha3_224::new()),
        FileDriver::Sha3_256 => Box::new(sha3::Sha3_256::new()),
        FileDriver::Sha3_384 => Box::new(sha3::Sha3_384::new()),
        FileDriver::Sha3_512 => Box::new(sha3::Sha3_512::new()),
    };

    hasher.update(data);
    let result = hasher.finalize_reset();
    Ok(hex::encode(result))
}

pub fn verify_file(expected_hex: &str, data: &[u8]) -> anyhow::Result<bool> {
    let computed = hash_file(data)?;
    Ok(computed.eq_ignore_ascii_case(expected_hex))
}

pub async fn verify_hash_file(file_path: &str) -> anyhow::Result<bool> {
    let hash_path = format!("{}.hash", file_path);
    let expected_hash = tokio::fs::read_to_string(&hash_path).await?.trim().to_string();
    let file_bytes = tokio::fs::read(file_path).await?;
    verify_file(&expected_hash, &file_bytes)
}

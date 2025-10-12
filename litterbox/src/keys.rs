use argon2::Argon2;
use russh::keys::{Algorithm, PrivateKey};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

use crate::{LitterboxError, files::keyfile_path};

fn gen_key() -> PrivateKey {
    use russh::keys::signature::rand_core::OsRng;

    let mut rng = OsRng::default();

    // FIXME: return an error instead of unwrapping
    PrivateKey::random(&mut rng, Algorithm::Ed25519).expect("Algorithm should be known.")
}

fn hash_password(password: &str) -> String {
    use argon2::password_hash::{PasswordHasher, SaltString, rand_core::OsRng};

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    // FIXME: return error instead of crashing
    argon2
        .hash_password(password.as_bytes(), &salt)
        .unwrap()
        .to_string()
}

fn check_password(password: &str, hash: &str) -> bool {
    use argon2::password_hash::{PasswordHash, PasswordVerifier};

    // FIXME: return error instead of crashing
    let parsed_hash = PasswordHash::new(&hash).unwrap();

    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

#[derive(Debug, Deserialize, Serialize)]
struct Key {
    name: String,
    encrypted_key: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Keys {
    password_hash: String,
    keys: Vec<Key>,
}

// TODO: perhaps we should place a lock on the keyfile while this struct exists?

impl Keys {
    fn save_to_file(&self) {
        // FIXME: return error instead of unwrapping
        let contents = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default()).unwrap();

        todo!()
    }

    pub fn init_default() -> Result<Self, LitterboxError> {
        todo!()
    }

    pub fn read_from_file() -> Result<Self, LitterboxError> {
        let path = keyfile_path()?;

        if Path::new(&path).exists() {
            return Self::init_default();
        }

        let contents =
            fs::read_to_string(&path).map_err(|e| LitterboxError::ReadFailed(e, path))?;

        // FIXME: return error instead of unwrapping
        let parsed: Self = ron::from_str(&contents).unwrap();
        Ok(parsed)
    }

    pub fn print_list(&self) {}
}

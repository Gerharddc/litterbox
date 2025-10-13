use argon2::Argon2;
use inquire::Password;
use russh::keys::{Algorithm, PrivateKey, pkcs8::encode_pkcs8_encrypted};
use serde::{Deserialize, Serialize};

use crate::{
    LitterboxError,
    files::{keyfile_path, read_file, write_file},
};

fn gen_key() -> PrivateKey {
    use russh::keys::signature::rand_core::OsRng;

    // FIXME: return an error instead of unwrapping
    PrivateKey::random(&mut OsRng, Algorithm::Ed25519).expect("Algorithm should be known.")
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
    let parsed_hash = PasswordHash::new(hash).unwrap();

    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

#[derive(Debug, Deserialize, Serialize)]
struct Key {
    name: String,
    encrypted_key: Vec<u8>,
    attached_litterboxes: Vec<String>,
}

impl Key {
    fn new(name: &str, password: &str) -> Self {
        let key = gen_key();

        // FIXME: return error instead of crashing
        let encrypted_key = encode_pkcs8_encrypted(password.as_bytes(), 10, &key).unwrap();

        Self {
            name: name.to_owned(),
            encrypted_key,
            attached_litterboxes: Vec::new(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Keys {
    password_hash: String,
    keys: Vec<Key>,
}

// TODO: perhaps we should place a lock on the keyfile while this struct exists?

impl Keys {
    fn save_to_file(&self) -> Result<(), LitterboxError> {
        let path = keyfile_path()?;
        // let contents = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
        let contents = ron::ser::to_string(self).map_err(|e| {
            eprintln!("Serialise error: {:#?}", e);
            LitterboxError::FailedToSerialise("Keys")
        })?;
        write_file(path.as_path(), &contents)
    }

    pub fn init_default() -> Result<Self, LitterboxError> {
        println!("Please enter a password to protect your keys.");
        let password = Password::new("Key Manager Password")
            .with_display_mode(inquire::PasswordDisplayMode::Masked)
            .prompt()
            .map_err(LitterboxError::PromptError)?;

        let password_hash = hash_password(&password);
        let keys = Vec::new();
        let s = Self {
            password_hash,
            keys,
        };

        s.save_to_file()?;
        Ok(s)
    }

    pub fn load() -> Result<Self, LitterboxError> {
        let keyfile = keyfile_path()?;
        if !keyfile.exists() {
            println!("Keys file does not exist yet. A new one will be created.");
            return Self::init_default();
        }

        let contents = read_file(keyfile.as_path())?;

        // FIXME: return error instead of unwrapping
        let parsed: Self = ron::from_str(&contents).unwrap();
        Ok(parsed)
    }

    pub fn print_list(&self) {
        // List all keys and the containers they are assigned to
        todo!()
    }

    fn prompt_password(&self) -> Result<String, LitterboxError> {
        println!("Please enter the password you chose for the key manager.");
        loop {
            let password = Password::new("Key Manager Password")
                .with_display_mode(inquire::PasswordDisplayMode::Masked)
                .prompt()
                .map_err(LitterboxError::PromptError)?;

            if check_password(&password, &self.password_hash) {
                return Ok(password);
            } else {
                println!("The provided password was not correct. Please try again.");
            }
        }
    }

    pub fn generate(&mut self, key_name: &str) -> Result<(), LitterboxError> {
        // FIXME: make sure a key with this name does not exist yet

        let password = self.prompt_password()?;
        self.keys.push(Key::new(key_name, &password));
        self.save_to_file()?;
        Ok(())
    }

    pub fn delete(&mut self, key_name: &str) -> Result<(), LitterboxError> {
        let mut found = false;
        self.keys.retain(|k| {
            if k.name == key_name {
                found = true;
                false
            } else {
                true
            }
        });

        if found {
            self.save_to_file()?;
            println!("Deleted key named {key_name}");
        } else {
            println!("Could not find key named {key_name}. Nothing deleted.")
        }

        Ok(())
    }
}

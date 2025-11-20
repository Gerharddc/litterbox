use argon2::Argon2;
use inquire::{MultiSelect, Password};
use russh::keys::{Algorithm, PrivateKey, pkcs8::decode_pkcs8, pkcs8::encode_pkcs8_encrypted};
use serde::{Deserialize, Serialize};
use tabled::{Table, Tabled};

use crate::{
    LitterboxError,
    files::{keyfile_path, read_file, write_file},
};

fn gen_key() -> PrivateKey {
    use russh::keys::signature::rand_core::OsRng;
    PrivateKey::random(&mut OsRng, Algorithm::Ed25519).expect("Ed25519 should be supported.")
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

#[derive(Tabled)]
struct KeyTableRow {
    name: String,
    attached_litterboxes: String,
}

impl From<&Key> for KeyTableRow {
    fn from(value: &Key) -> Self {
        Self {
            name: value.name.clone(),
            attached_litterboxes: value.attached_litterboxes.join(","),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Keys {
    password_hash: String,
    keys: Vec<Key>,
}

impl Keys {
    // TODO: perhaps we should place a lock on the keyfile while this struct exists?

    fn save_to_file(&self) -> Result<(), LitterboxError> {
        let path = keyfile_path()?;
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
        let table_rows: Vec<KeyTableRow> = self.keys.iter().map(|c| c.into()).collect();
        let table = Table::new(table_rows);
        println!("{table}");
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

    fn key_mut(&mut self, key_name: &str) -> Option<&mut Key> {
        self.keys.iter_mut().find(|key| key.name == key_name)
    }

    pub fn generate(&mut self, key_name: &str) -> Result<(), LitterboxError> {
        if self.key_mut(key_name).is_some() {
            return Err(LitterboxError::KeyAlreadyExists(key_name.to_owned()));
        }

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

        if !found {
            return Err(LitterboxError::KeyDoesNotExist(key_name.to_owned()));
        }

        self.save_to_file()?;
        println!("Deleted key named {key_name}");
        Ok(())
    }

    pub fn attach(&mut self, key_name: &str, litterbox_name: &str) -> Result<(), LitterboxError> {
        match self.key_mut(key_name) {
            Some(key) => {
                if key
                    .attached_litterboxes
                    .iter()
                    .any(|name| *name == litterbox_name)
                {
                    return Err(LitterboxError::AlreadyAttachedToKey(
                        key_name.to_owned(),
                        litterbox_name.to_owned(),
                    ));
                }

                key.attached_litterboxes.push(litterbox_name.to_owned());
                self.save_to_file()?;

                println!("Attached {litterbox_name} to {key_name}!");
                Ok(())
            }
            None => Err(LitterboxError::KeyDoesNotExist(key_name.to_owned())),
        }
    }

    pub fn detach(&mut self, key_name: &str) -> Result<(), LitterboxError> {
        match self.key_mut(key_name) {
            Some(key) => {
                let to_remove = MultiSelect::new(
                    "Select the Litterboxes that you want to detach:",
                    key.attached_litterboxes.clone(),
                )
                .prompt()
                .map_err(LitterboxError::PromptError)?;

                key.attached_litterboxes
                    .retain(|name| !to_remove.contains(name));

                self.save_to_file()?;
                println!("Detached {} Litterbox from {key_name}!", to_remove.len());
                println!("N.B. running Litterboxes won't be affected until they are restarted!!");
                Ok(())
            }
            None => Err(LitterboxError::KeyDoesNotExist(key_name.to_owned())),
        }
    }

    pub async fn start_server(&self, lbx_name: &str) -> Result<AskAgent, LitterboxError> {
        let agent_path = crate::agent::start_agent().await;
        let password = self.prompt_password()?;
        let keys = self
            .keys
            .iter()
            .filter(|key| key.attached_litterboxes.iter().any(|name| name == lbx_name));

        let stream = tokio::net::UnixStream::connect(&agent_path)
            .await
            .map_err(LitterboxError::ConnectSocket)?;
        let mut client = russh::keys::agent::client::AgentClient::connect(stream);

        for key in keys {
            println!("Registering key: {}", key.name);

            let decrypted = decode_pkcs8(&key.encrypted_key, Some(password.as_bytes()))
                .expect("Key should have been encrypted with user password.");

            client
                .add_identity(&decrypted, &[])
                .await
                .map_err(LitterboxError::RegisterKey)?;
        }

        Ok(AskAgent {})
    }
}

pub struct AskAgent {}

impl Drop for AskAgent {
    fn drop(&mut self) {
        println!("Killing SSH key server");

        // FIXME: implement if needed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_hash_and_verify_password() {
        let password = "some_random_pass";
        let hash = hash_password(password);
        assert_ne!(password, &hash);

        assert!(check_password(password, &hash));
        assert!(!check_password("wrong_pass", &hash));
    }

    #[test]
    fn can_encrypt_and_decrypt_password() {
        let password = "SomePassword";

        let original_key = gen_key();

        let encrypted_key = encode_pkcs8_encrypted(password.as_bytes(), 10, &original_key).unwrap();

        let decrypted_key = decode_pkcs8(&encrypted_key, Some(password.as_bytes()))
            .expect("Key should have been encrypted with user password.");

        assert_eq!(decrypted_key, original_key);
    }
}

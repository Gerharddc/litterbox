use anyhow::{Context, Result, anyhow};
use argon2::Argon2;
use inquire::{MultiSelect, Password};
use log::info;
use nix::sys::signal::kill;
use nix::unistd::Pid;
use russh::keys::{
    Algorithm, PrivateKey,
    pkcs8::{decode_pkcs8, encode_pkcs8_encrypted},
    ssh_key::LineEnding,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, atomic::Ordering};
use tabled::{Table, Tabled};

use crate::{
    agent::{AgentState, start_ssh_agent},
    files,
    podman::is_container_running,
};

fn gen_key() -> PrivateKey {
    use russh::keys::signature::rand_core::OsRng;
    PrivateKey::random(&mut OsRng, Algorithm::Ed25519).expect("Ed25519 should be supported.")
}

fn hash_password(password: &str) -> String {
    use argon2::password_hash::{PasswordHasher, SaltString, rand_core::OsRng};

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    argon2
        .hash_password(password.as_bytes(), &salt)
        .expect("Passwords should be hashable")
        .to_string()
}

fn check_password(password: &str, hash: &str) -> bool {
    use argon2::password_hash::{PasswordHash, PasswordVerifier};

    let parsed_hash = PasswordHash::new(hash).expect("Passwords should have valid hashes");

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
        Self {
            name: name.to_owned(),
            encrypted_key: Self::encrypt(&gen_key(), password),
            attached_litterboxes: Vec::new(),
        }
    }

    fn encrypt(private_key: &PrivateKey, password: &str) -> Vec<u8> {
        encode_pkcs8_encrypted(password.as_bytes(), 10, private_key)
            .expect("Keys should be encryptable")
    }

    fn decrypt(&self, password: &str) -> PrivateKey {
        decode_pkcs8(&self.encrypted_key, Some(password.as_bytes()))
            .expect("Key should have been encrypted with user password.")
    }

    fn change_password(&mut self, old_password: &str, new_password: &str) {
        let decrypted = self.decrypt(old_password);
        self.encrypted_key = Self::encrypt(&decrypted, new_password);
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

    fn save_to_file(&self) -> Result<()> {
        let path = files::keyfile_path()?;
        let contents = ron::ser::to_string(self).context("failed to serialise keys")?;
        files::write_file(&path, &contents)
    }

    pub fn init_default() -> Result<Self> {
        println!("Please enter a password to protect your keys.");
        let password = Password::new("Key Manager Password")
            .with_display_mode(inquire::PasswordDisplayMode::Masked)
            .prompt()?;

        let password_hash = hash_password(&password);
        let keys = Vec::new();
        let s = Self {
            password_hash,
            keys,
        };

        s.save_to_file()?;
        Ok(s)
    }

    pub fn load() -> Result<Self> {
        let keyfile = files::keyfile_path()?;
        if !keyfile.exists() {
            println!("Keys file does not exist yet. A new one will be created.");
            return Self::init_default();
        }

        let contents = files::read_file(keyfile.as_path())?;
        Ok(ron::from_str(&contents)?)
    }

    pub fn print_list(&self) {
        let table_rows: Vec<KeyTableRow> = self.keys.iter().map(|c| c.into()).collect();
        let table = Table::new(table_rows);
        println!("{table}");
    }

    pub fn change_password(&mut self) -> Result<()> {
        let old_password = self.prompt_password()?;
        let new_password = Password::new("New Key Manager Password")
            .with_display_mode(inquire::PasswordDisplayMode::Masked)
            .prompt()?;

        for key in &mut self.keys {
            key.change_password(&old_password, &new_password);
        }
        self.password_hash = hash_password(&new_password);
        self.save_to_file()?;
        Ok(())
    }

    fn prompt_password(&self) -> Result<String> {
        println!("Please enter the password you chose for the key manager.");
        loop {
            let password = Password::new("Key Manager Password")
                .with_display_mode(inquire::PasswordDisplayMode::Masked)
                .without_confirmation()
                .prompt()?;

            if check_password(&password, &self.password_hash) {
                return Ok(password);
            } else {
                println!("The provided password was not correct. Please try again.");
            }
        }
    }

    fn key(&self, key_name: &str) -> Option<&Key> {
        self.keys.iter().find(|key| key.name == key_name)
    }

    fn key_mut(&mut self, key_name: &str) -> Option<&mut Key> {
        self.keys.iter_mut().find(|key| key.name == key_name)
    }

    pub fn generate(&mut self, key_name: &str) -> Result<()> {
        if self.key_mut(key_name).is_some() {
            return Err(anyhow!("Key {} already exists", key_name));
        }

        let password = self.prompt_password()?;
        self.keys.push(Key::new(key_name, &password));
        self.save_to_file()?;
        Ok(())
    }

    pub fn delete(&mut self, key_name: &str) -> Result<()> {
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
            return Err(anyhow!("Key {} does not exist", key_name));
        }

        self.save_to_file()?;
        println!("Deleted key named {key_name}");
        Ok(())
    }

    pub fn attach(&mut self, key_name: &str, litterbox_name: &str) -> Result<()> {
        match self.key_mut(key_name) {
            Some(key) => {
                if key
                    .attached_litterboxes
                    .iter()
                    .any(|name| *name == litterbox_name)
                {
                    return Err(anyhow!(
                        "Key {} already attached to litterbox {}",
                        key_name,
                        litterbox_name
                    ));
                }

                key.attached_litterboxes.push(litterbox_name.to_owned());
                self.save_to_file()?;

                println!("Attached {litterbox_name} to {key_name}!");
                Ok(())
            }
            None => Err(anyhow!("Key {} does not exist", key_name)),
        }
    }

    pub fn detach(&mut self, key_name: &str) -> Result<()> {
        match self.key_mut(key_name) {
            Some(key) => {
                let to_remove = MultiSelect::new(
                    "Select the Litterboxes that you want to detach:",
                    key.attached_litterboxes.clone(),
                )
                .prompt()?;

                key.attached_litterboxes
                    .retain(|name| !to_remove.contains(name));

                self.save_to_file()?;
                println!("Detached {} Litterbox from {key_name}!", to_remove.len());
                println!("N.B. running Litterboxes won't be affected until they are restarted!!");
                Ok(())
            }
            None => Err(anyhow!("Key {} does not exist", key_name)),
        }
    }

    fn attached_keys(&self, lbx_name: &str) -> Vec<&Key> {
        self.keys
            .iter()
            .filter(|key| key.attached_litterboxes.iter().any(|name| name == lbx_name))
            .collect()
    }

    fn has_attached_keys(&self, lbx_name: &str) -> bool {
        !self.attached_keys(lbx_name).is_empty()
    }

    pub fn password_if_needed(&self, lbx_name: &str) -> Result<Option<String>> {
        if self.has_attached_keys(lbx_name) {
            let password = self.prompt_password()?;
            Ok(Some(password))
        } else {
            Ok(None)
        }
    }

    pub async fn start_ssh_server(&self, lbx_name: &str, password: &str) -> Result<()> {
        let agent_state = Arc::new(AgentState::default());
        let agent_path = start_ssh_agent(lbx_name, agent_state.clone()).await?;
        log::debug!("agent_path: {:#?}", agent_path);

        let stream = tokio::net::UnixStream::connect(&agent_path)
            .await
            .context("Failed to connect to SSH agent socket")?;
        let mut client = russh::keys::agent::client::AgentClient::connect(stream);

        log::debug!("Registering keys to SSH agent.");
        for key in self.attached_keys(lbx_name) {
            log::info!("Registering key into agent: {}", key.name);

            let decrypted = key.decrypt(password);
            client
                .add_identity(&decrypted, &[])
                .await
                .context("Failed to register SSH key")?;
        }

        // Ensure the agent will now start prompting for authorization
        agent_state.locked.store(true, Ordering::SeqCst);

        Ok(())
    }

    pub fn print(&self, key_name: &str, private: bool) -> Result<()> {
        match self.key(key_name) {
            Some(key) => {
                let keys_password = self.prompt_password()?;
                let decrypted = key.decrypt(&keys_password);

                let openssh = if private {
                    decrypted
                        .to_openssh(LineEnding::LF)
                        .expect("OpenSSH format key should be valid.")
                } else {
                    let public = decrypted.public_key();
                    public
                        .to_openssh()
                        .expect("OpenSSH format key should be valid.")
                        .into()
                };

                println!("{}", openssh.as_str());
                Ok(())
            }
            None => Err(anyhow!("Key {} does not exist", key_name)),
        }
    }
}

// TODO: keys.rs is probably not the best place for this function anymore
pub async fn run_daemon(lbx_name: &str, password: &str) -> Result<()> {
    let daemon_lock = files::daemon_lock_path(lbx_name)?;

    if daemon_lock.exists() {
        let pid_str =
            std::fs::read_to_string(&daemon_lock).context("Failed to read daemon lock file")?;

        if let Ok(pid) = pid_str.trim().parse::<u32>() {
            let pid = Pid::from_raw(pid as i32);
            if kill(pid, None).is_ok() {
                info!("Daemon already running for {}", lbx_name);
                return Ok(());
            }
        }

        info!("Stale daemon lock file found, removing");
        std::fs::remove_file(&daemon_lock).context("Failed to remove stale daemon lock file")?;
    }

    let my_pid = std::process::id();
    std::fs::write(&daemon_lock, my_pid.to_string()).context("Failed to write daemon lock file")?;

    let keys = Keys::load()?;
    keys.start_ssh_server(lbx_name, password).await?;

    let session_path = files::session_lock_path(lbx_name)?;

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

        files::cleanup_dead_pids_from_session_lockfile(&session_path)?;

        if !is_container_running(lbx_name)? {
            info!("Container no longer running, daemon will stop.");
            break;
        }
    }

    if session_path.exists() {
        info!("Cleaning up session lockfile.");
        std::fs::remove_file(&session_path).context("Failed to remove session lock file")?;
    }

    std::fs::remove_file(&daemon_lock).context("Failed to remove daemon lock file")?;
    info!("Daemon exiting for {}", lbx_name);
    Ok(())
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

        let encrypted_key = Key {
            name: String::new(),
            encrypted_key: Key::encrypt(&original_key, password),
            attached_litterboxes: Vec::new(),
        };
        let decrypted_key = encrypted_key.decrypt(password);
        assert_eq!(decrypted_key, original_key);
    }
}

use argon2::Argon2;
use russh::keys::{Algorithm, PrivateKey};

fn gen_key() -> PrivateKey {
    use russh::keys::signature::rand_core::OsRng;

    let mut rng = OsRng::default();

    // FIXME: return an error instead of unwrapping
    PrivateKey::random(&mut rng, Algorithm::Ed25519).unwrap()
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

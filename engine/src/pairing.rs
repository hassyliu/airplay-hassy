use aes::Aes128;
use cipher::{KeyIvInit, StreamCipher};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use sha2::{Digest, Sha512};
use x25519_dalek::{PublicKey as X25519Public, StaticSecret as X25519Secret};

type Aes128Ctr = ctr::Ctr128BE<Aes128>;

const SALT_KEY: &[u8] = b"Pair-Verify-AES-Key";
const SALT_IV: &[u8] = b"Pair-Verify-AES-IV";

#[derive(Clone)]
pub struct PairingIdentity {
    pub signing_key: SigningKey,
}

impl PairingIdentity {
    pub fn generate() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        Self { signing_key }
    }

    pub fn public_key_bytes(&self) -> [u8; 32] {
        self.signing_key.verifying_key().to_bytes()
    }

    pub fn public_key_hex(&self) -> String {
        hex::encode(self.public_key_bytes())
    }
}

pub struct PairingSession {
    ed_ours: SigningKey,
    ed_theirs: Option<VerifyingKey>,
    ecdh_ours: Option<X25519Secret>,
    ecdh_ours_pub: Option<[u8; 32]>,
    ecdh_theirs_pub: Option<[u8; 32]>,
    ecdh_secret: Option<[u8; 32]>,
    verified: bool,
}

impl PairingSession {
    pub fn new(identity: &PairingIdentity) -> Self {
        Self {
            ed_ours: identity.signing_key.clone(),
            ed_theirs: None,
            ecdh_ours: None,
            ecdh_ours_pub: None,
            ecdh_theirs_pub: None,
            ecdh_secret: None,
            verified: false,
        }
    }

    pub fn ecdh_secret(&self) -> Option<[u8; 32]> {
        self.ecdh_secret
    }

    fn derive_aes_key_iv(&self) -> Option<([u8; 16], [u8; 16])> {
        let secret = self.ecdh_secret?;
        let mut h_key = Sha512::new();
        h_key.update(SALT_KEY);
        h_key.update(secret);
        let d_key = h_key.finalize();

        let mut h_iv = Sha512::new();
        h_iv.update(SALT_IV);
        h_iv.update(secret);
        let d_iv = h_iv.finalize();

        let mut key = [0u8; 16];
        let mut iv = [0u8; 16];
        key.copy_from_slice(&d_key[..16]);
        iv.copy_from_slice(&d_iv[..16]);
        Some((key, iv))
    }

    /// Stage 1 of `/pair-verify`: client sends `[1, 0, 0, 0] + client_ecdh_pub(32) + client_ed_pub(32)`
    /// Returns `server_ecdh_pub(32) + encrypted_signature(64)` = 96 bytes.
    pub fn verify_step1(&mut self, data: &[u8]) -> Result<Vec<u8>, String> {
        if data.len() != 68 || data[0] != 1 {
            return Err(format!("Invalid pair-verify step1 length {}", data.len()));
        }
        let mut client_ecdh = [0u8; 32];
        let mut client_ed = [0u8; 32];
        client_ecdh.copy_from_slice(&data[4..36]);
        client_ed.copy_from_slice(&data[36..68]);

        let their_x = X25519Public::from(client_ecdh);
        let their_ed = VerifyingKey::from_bytes(&client_ed)
            .map_err(|e| format!("Invalid client Ed25519 key: {}", e))?;

        let our_secret = X25519Secret::random_from_rng(OsRng);
        let our_pub = X25519Public::from(&our_secret);
        let shared = our_secret.diffie_hellman(&their_x);

        self.ecdh_ours = Some(our_secret);
        self.ecdh_ours_pub = Some(*our_pub.as_bytes());
        self.ecdh_theirs_pub = Some(client_ecdh);
        self.ed_theirs = Some(their_ed);
        self.ecdh_secret = Some(*shared.as_bytes());

        // Sign (our_ecdh_pub || their_ecdh_pub)
        let mut sig_msg = [0u8; 64];
        sig_msg[..32].copy_from_slice(our_pub.as_bytes());
        sig_msg[32..].copy_from_slice(&client_ecdh);
        let mut sig_bytes = self.ed_ours.sign(&sig_msg).to_bytes();

        // Encrypt signature using AES-128-CTR
        let (key, iv) = self.derive_aes_key_iv().ok_or("Missing ECDH secret")?;
        let mut cipher = Aes128Ctr::new(&key.into(), &iv.into());
        cipher.apply_keystream(&mut sig_bytes);

        let mut out = Vec::with_capacity(96);
        out.extend_from_slice(our_pub.as_bytes());
        out.extend_from_slice(&sig_bytes);
        Ok(out)
    }

    /// Stage 2 of `/pair-verify`: client sends `[0, 0, 0, 0] + encrypted_client_sig(64)`
    pub fn verify_step2(&mut self, data: &[u8]) -> Result<(), String> {
        if data.len() != 68 || data[0] != 0 {
            return Err(format!("Invalid pair-verify step2 length {}", data.len()));
        }
        let (key, iv) = self.derive_aes_key_iv().ok_or("Missing ECDH secret")?;
        let mut cipher = Aes128Ctr::new(&key.into(), &iv.into());
        // Advance CTR keystream past the 64 bytes encrypted in step 1
        let mut dummy = [0u8; 64];
        cipher.apply_keystream(&mut dummy);

        let mut sig_bytes = [0u8; 64];
        sig_bytes.copy_from_slice(&data[4..68]);
        cipher.apply_keystream(&mut sig_bytes);

        let their_pub = self.ecdh_theirs_pub.ok_or("Missing client ECDH")?;
        let our_pub = self.ecdh_ours_pub.ok_or("Missing server ECDH")?;
        let their_ed = self.ed_theirs.ok_or("Missing client Ed25519")?;

        let mut sig_msg = [0u8; 64];
        sig_msg[..32].copy_from_slice(&their_pub);
        sig_msg[32..].copy_from_slice(&our_pub);

        let sig = Signature::from_bytes(&sig_bytes);
        their_ed
            .verify(&sig_msg, &sig)
            .map_err(|e| format!("Client pair-verify signature invalid: {}", e))?;

        self.verified = true;
        Ok(())
    }
}

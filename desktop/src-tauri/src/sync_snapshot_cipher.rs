use aes_gcm::aead::{Aead, AeadCore, KeyInit};
use aes_gcm::{Aes256Gcm, Key};
use hkdf::Hkdf;
use rand::rngs::OsRng;
use sha2::Sha256;

/// Fase 8.4, Sessão 87 — cifra do snapshot do banco *em repouso*, antes de
/// pinar (diferente de `pin_content_cipher.rs`, que só cifra o *transporte*
/// cross-device). A chave vem dos bytes de uma assinatura `personal_sign`
/// devolvida por `/truthid/v1/sign-message` (loopback ou cross-device,
/// `commands/truthid.rs`) — como a assinatura é determinística pra uma dada
/// chave privada + mensagem, pedir de novo sempre deriva a mesma chave, sem
/// o Anchor persistir segredo nenhum. `HKDF_SALT`/`HKDF_INFO` são próprios
/// deste módulo — nunca reusar os de `pin_content_cipher.rs`/`ipns_key.rs`,
/// domain separation (mesmo cuidado já documentado nos dois).
const HKDF_SALT: &[u8] = b"Anchor Sync Snapshot";
const HKDF_INFO: &[u8] = b"snapshot-key-v1";

const NONCE_LEN: usize = 12;

/// Deriva a chave AES-256 a partir dos bytes crus de uma assinatura ECDSA.
pub fn derive_key(signature_bytes: &[u8]) -> [u8; 32] {
    let hk = Hkdf::<Sha256>::new(Some(HKDF_SALT), signature_bytes);
    let mut key = [0u8; 32];
    hk.expand(HKDF_INFO, &mut key)
        .expect("32 bytes always fits a single HKDF-SHA256 expand");
    key
}

/// Cifra o snapshot. Mesmo layout de `pin_content_cipher.rs::encrypt`
/// (`nonce(12) || ciphertext+tag`, AES-256-GCM) — infalível pelo mesmo motivo
/// (chave de 32 bytes nunca produz erro de tamanho no `aes-gcm`).
pub fn encrypt(plaintext: &[u8], key: &[u8; 32]) -> Vec<u8> {
    let aes_key = Key::<Aes256Gcm>::from_slice(key);
    let cipher = Aes256Gcm::new(aes_key);
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, plaintext)
        .expect("AES-256-GCM encrypt with a valid 32-byte key never fails");

    let mut blob = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    blob.extend_from_slice(&nonce);
    blob.extend_from_slice(&ciphertext);
    blob
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sem consumidor de produção ainda — ler o snapshot pinado de volta é a
    /// fatia 3/4 da fila da Fase 8 (branch `ar://` + revalidação ponta a
    /// ponta). Mesmo padrão de `pin_content_cipher.rs::decrypt_for_test`.
    fn decrypt_for_test(blob: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, String> {
        if blob.len() < NONCE_LEN {
            return Err("blob too short".to_string());
        }
        let nonce = &blob[..NONCE_LEN];
        let ciphertext = &blob[NONCE_LEN..];

        let aes_key = Key::<Aes256Gcm>::from_slice(key);
        let cipher = Aes256Gcm::new(aes_key);
        cipher
            .decrypt(nonce.into(), ciphertext)
            .map_err(|_| "decrypt failed".to_string())
    }

    #[test]
    fn round_trips_with_a_derived_key() {
        let key = derive_key(b"fake-ecdsa-signature-bytes");
        let plaintext = b"sqlite database snapshot bytes";

        let blob = encrypt(plaintext, &key);
        let decrypted = decrypt_for_test(&blob, &key).expect("should decrypt");

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn derivation_is_deterministic() {
        let a = derive_key(b"same-signature");
        let b = derive_key(b"same-signature");
        assert_eq!(a, b);
    }

    #[test]
    fn different_signatures_derive_different_keys() {
        let a = derive_key(b"signature-one");
        let b = derive_key(b"signature-two");
        assert_ne!(a, b);
    }
}

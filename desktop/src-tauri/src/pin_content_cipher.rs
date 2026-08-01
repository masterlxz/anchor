use aes_gcm::aead::{Aead, AeadCore, KeyInit};
use aes_gcm::{Aes256Gcm, Key};
use hkdf::Hkdf;
use rand::rngs::OsRng;
use sha2::Sha256;

/// Cifra simétrica do conteúdo a pinar, fase 1 do `/pin` cross-device — port
/// do lado requisitante de `mobile/lib/services/pin_content_cipher_service.dart`
/// (TruthID). Nesta direção (requisitante → celular) nenhuma das duas pontas
/// tem a chave pública da outra — o QR já saiu antes do celular entrar em
/// cena, e o celular não expõe nenhuma chave própria no payload do `/pin`.
/// Resolvido com uma chave simétrica derivada deterministicamente do
/// `sessionId` via HKDF (mesmo padrão que `ipns_key.rs` já usa pra derivar a
/// chave IPNS do dead-drop) — o `sessionId` (só conhecido por quem viu o QR)
/// já faz o papel de segredo compartilhado.
///
/// `HKDF_SALT`/`HKDF_INFO` precisam bater byte-a-byte com o lado Dart — nunca
/// reusar os da derivação IPNS (`ipns_key.rs`), domain separation.
const HKDF_SALT: &[u8] = b"TruthID Pin Content";
const HKDF_INFO: &[u8] = b"content-key-v1";

const NONCE_LEN: usize = 12;

/// Deriva a chave AES-256 da fase 1 a partir do `sessionId` (hex).
pub fn derive_pin_content_key(session_id_hex: &str) -> Result<[u8; 32], String> {
    let session_id_bytes = hex::decode(session_id_hex).map_err(|e| e.to_string())?;

    let hk = Hkdf::<Sha256>::new(Some(HKDF_SALT), &session_id_bytes);
    let mut key = [0u8; 32];
    hk.expand(HKDF_INFO, &mut key)
        .map_err(|_| "HKDF expand failed".to_string())?;

    Ok(key)
}

/// Cifra o conteúdo pro celular. Formato do blob: `nonce(12) || ciphertext+tag`
/// (AES-256-GCM, sem cabeçalho de chave pública — a chave já é simétrica,
/// diferente do formato ECIES que `ecies.rs` usa pra fase 2/resultado). Um
/// `[u8; 32]` como chave nunca produz erro de tamanho no `aes-gcm`, por isso
/// esta função é infalível.
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

    /// Réplica mínima do lado "decifra" (papel do celular, `decryptPinContent`
    /// no Dart) — só pra provar round-trip Rust↔Rust sem depender de um
    /// celular real. Mesmo espírito invertido de `ecies.rs::encrypt_for_test`
    /// (lá a produção é `decrypt` e o teste simula `encrypt`; aqui é o oposto).
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
        let key = derive_pin_content_key("000102030405060708090a0b0c0d0e0f").unwrap();
        let plaintext = b"cross-device pin content";

        let blob = encrypt(plaintext, &key);
        let decrypted = decrypt_for_test(&blob, &key).expect("should decrypt");

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn derivation_is_deterministic() {
        let a = derive_pin_content_key("000102030405060708090a0b0c0d0e0f").unwrap();
        let b = derive_pin_content_key("000102030405060708090a0b0c0d0e0f").unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn different_session_ids_derive_different_keys() {
        let a = derive_pin_content_key("000102030405060708090a0b0c0d0e0f").unwrap();
        let b = derive_pin_content_key("0f0e0d0c0b0a09080706050403020100").unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn rejects_invalid_hex() {
        assert!(derive_pin_content_key("not-hex").is_err());
    }

    // Vetor cruzado real, gerado uma vez rodando `derivePinContentKey`/
    // `encryptPinContent` de verdade no lado Dart
    // (`mobile/lib/services/pin_content_cipher_service.dart`) contra este
    // mesmo `sessionId`/plaintext — prova que os dois lados derivam
    // exatamente a mesma chave e o mesmo layout de blob (nonce||ciphertext+tag),
    // sem precisar de um celular físico pra pegar um mismatch de domain
    // separation ou de formato (mesma disciplina que já achou 2 bugs reais
    // nesta frente: o MAC do ECIES na Sessão 99, o camelCase na Sessão 114 —
    // nenhum dos dois pego por round-trip só-interno).
    #[test]
    fn decrypts_a_fixture_encrypted_by_the_real_dart_side() {
        let session_id_hex = "000102030405060708090a0b0c0d0e0f";
        let key = derive_pin_content_key(session_id_hex).unwrap();

        // Gerado rodando de verdade `derivePinContentKey`/`encryptPinContent`
        // no Dart (`tool/pin_content_cipher_fixture.dart`, repo TruthID) —
        // mesmo blob que `pin_content_cipher_service_test.dart` usa do outro
        // lado.
        let blob_hex = "b23757d2e7de00df20298fa375385826e472958faa49cd2320c8694c7770\
e7a6415cf7d6a6b72ae251e79001cb9b1dc4eb57f2b9fcdd2c";
        let expected_plaintext = b"truthid-pin-content-fixture";

        let blob = hex::decode(blob_hex).expect("valid hex fixture");
        let decrypted =
            decrypt_for_test(&blob, &key).expect("should decrypt the Dart-produced blob");

        assert_eq!(decrypted, expected_plaintext);
    }
}

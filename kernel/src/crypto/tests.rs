#![cfg(test)]

use super::*;
use alloc::vec;

#[test]
fn test_aes_encryption_decryption() {
    let engine = CryptoEngine::new();
    let cipher = engine.get_cipher(CipherAlgorithm::Aes256).unwrap();
    
    let key = vec![0x42u8; 32];
    let iv = vec![0x00u8; 16];
    let plaintext = b"Hello, Crypto World!";
    
    let ciphertext = cipher.encrypt(plaintext, &key, Some(&iv)).unwrap();
    assert_ne!(ciphertext, plaintext);
    
    let decrypted = cipher.decrypt(&ciphertext, &key, Some(&iv)).unwrap();
    assert_eq!(decrypted, plaintext);
}

#[test]
fn test_chacha20_encryption() {
    let engine = CryptoEngine::new();
    let cipher = engine.get_cipher(CipherAlgorithm::ChaCha20).unwrap();
    
    let key = vec![0x01u8; 32];
    let nonce = vec![0x00u8; 12];
    let plaintext = b"ChaCha20 test message";
    
    let ciphertext = cipher.encrypt(plaintext, &key, Some(&nonce)).unwrap();
    let decrypted = cipher.decrypt(&ciphertext, &key, Some(&nonce)).unwrap();
    
    assert_eq!(decrypted, plaintext);
}

#[test]
fn test_sha256_hash() {
    let engine = CryptoEngine::new();
    let hasher = engine.get_hash(HashAlgorithm::SHA256).unwrap();
    
    let data = b"Test data for SHA-256";
    let hash = hasher.hash(data);
    
    assert_eq!(hash.len(), 32);
    
    let hash2 = hasher.hash(data);
    assert_eq!(hash, hash2);
}

#[test]
fn test_hmac_sha256() {
    let engine = CryptoEngine::new();
    let mac = engine.get_mac(MacAlgorithm::HmacSHA256).unwrap();
    
    let key = b"secret key";
    let data = b"message to authenticate";
    
    let tag = mac.compute(key, data).unwrap();
    assert_eq!(tag.len(), 32);
    
    let valid = mac.verify(key, data, &tag).unwrap();
    assert!(valid);
    
    let invalid = mac.verify(b"wrong key", data, &tag).unwrap();
    assert!(!invalid);
}

#[test]
fn test_chacha20_poly1305_aead() {
    let engine = CryptoEngine::new();
    let aead = engine.get_aead(AeadAlgorithm::ChaCha20Poly1305).unwrap();
    
    let key = vec![0x42u8; 32];
    let nonce = vec![0x00u8; 12];
    let plaintext = b"Authenticated encryption test";
    let aad = b"Additional authenticated data";
    
    let ciphertext = aead.encrypt(&key, &nonce, plaintext, aad).unwrap();
    assert_eq!(ciphertext.len(), plaintext.len() + 16);
    
    let decrypted = aead.decrypt(&key, &nonce, &ciphertext, aad).unwrap();
    assert_eq!(decrypted, plaintext);
    
    let bad_aad = b"Wrong AAD";
    let result = aead.decrypt(&key, &nonce, &ciphertext, bad_aad);
    assert!(result.is_err());
}

#[test]
fn test_pbkdf2_key_derivation() {
    let engine = CryptoEngine::new();
    let kdf = engine.get_kdf(KdfAlgorithm::PBKDF2SHA256).unwrap();
    
    let password = b"password123";
    let salt = b"saltsalt";
    let iterations = 1000;
    let key_len = 32;
    
    let key1 = kdf.derive(password, salt, iterations, key_len).unwrap();
    assert_eq!(key1.len(), key_len);
    
    let key2 = kdf.derive(password, salt, iterations, key_len).unwrap();
    assert_eq!(key1, key2);
    
    let key3 = kdf.derive(b"different", salt, iterations, key_len).unwrap();
    assert_ne!(key1, key3);
}

#[test]
fn test_secure_random() {
    let engine = CryptoEngine::new();
    let rng = engine.get_random();
    
    let random1 = rng.generate(32);
    let random2 = rng.generate(32);
    
    assert_eq!(random1.len(), 32);
    assert_eq!(random2.len(), 32);
    assert_ne!(random1, random2);
    
    let range_val = rng.generate_range(100, 200);
    assert!(range_val >= 100 && range_val < 200);
}

#[test]
fn test_ed25519_signing() {
    let engine = CryptoEngine::new();
    let asymmetric = engine.get_asymmetric(AsymmetricAlgorithm::Ed25519).unwrap();
    
    let (public_key, private_key) = asymmetric.generate_keypair().unwrap();
    let message = b"Message to sign";
    
    let signature = asymmetric.sign(&private_key, message).unwrap();
    assert_eq!(signature.len(), 64);
    
    let valid = asymmetric.verify(&public_key, message, &signature).unwrap();
    assert!(valid);
}

#[test]
fn test_rsa_encryption() {
    let engine = CryptoEngine::new();
    let rsa = engine.get_asymmetric(AsymmetricAlgorithm::RSA2048).unwrap();
    
    let (public_key, private_key) = rsa.generate_keypair().unwrap();
    let plaintext = b"RSA encryption test";
    
    let ciphertext = rsa.encrypt(&public_key, plaintext).unwrap();
    let decrypted = rsa.decrypt(&private_key, &ciphertext).unwrap();
    
    assert_eq!(decrypted, plaintext);
}
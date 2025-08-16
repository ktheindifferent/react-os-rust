#![no_std]

use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CryptoError {
    InvalidKeySize,
    InvalidBlockSize,
    InvalidNonce,
    InvalidTag,
    AuthenticationFailed,
    DecryptionFailed,
    UnsupportedAlgorithm,
    HardwareError,
    InvalidParameter,
    BufferTooSmall,
    NotInitialized,
    AlreadyInitialized,
    InvalidPadding,
    InvalidSignature,
    KeyGenerationFailed,
    RandomGeneratorFailed,
    CertificateError,
    InvalidCertificate,
    ExpiredCertificate,
    UntrustedCertificate,
    InvalidKeyFormat,
    UnsupportedKeyType,
    PermissionDenied,
    ResourceExhausted,
    InvalidState,
}

impl fmt::Display for CryptoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidKeySize => write!(f, "Invalid key size"),
            Self::InvalidBlockSize => write!(f, "Invalid block size"),
            Self::InvalidNonce => write!(f, "Invalid nonce"),
            Self::InvalidTag => write!(f, "Invalid authentication tag"),
            Self::AuthenticationFailed => write!(f, "Authentication failed"),
            Self::DecryptionFailed => write!(f, "Decryption failed"),
            Self::UnsupportedAlgorithm => write!(f, "Unsupported algorithm"),
            Self::HardwareError => write!(f, "Hardware crypto error"),
            Self::InvalidParameter => write!(f, "Invalid parameter"),
            Self::BufferTooSmall => write!(f, "Buffer too small"),
            Self::NotInitialized => write!(f, "Crypto not initialized"),
            Self::AlreadyInitialized => write!(f, "Already initialized"),
            Self::InvalidPadding => write!(f, "Invalid padding"),
            Self::InvalidSignature => write!(f, "Invalid signature"),
            Self::KeyGenerationFailed => write!(f, "Key generation failed"),
            Self::RandomGeneratorFailed => write!(f, "Random generator failed"),
            Self::CertificateError => write!(f, "Certificate error"),
            Self::InvalidCertificate => write!(f, "Invalid certificate"),
            Self::ExpiredCertificate => write!(f, "Expired certificate"),
            Self::UntrustedCertificate => write!(f, "Untrusted certificate"),
            Self::InvalidKeyFormat => write!(f, "Invalid key format"),
            Self::UnsupportedKeyType => write!(f, "Unsupported key type"),
            Self::PermissionDenied => write!(f, "Permission denied"),
            Self::ResourceExhausted => write!(f, "Resource exhausted"),
            Self::InvalidState => write!(f, "Invalid state"),
        }
    }
}

pub type CryptoResult<T> = Result<T, CryptoError>;
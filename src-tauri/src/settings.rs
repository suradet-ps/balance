//! Encrypted settings management.
//!
//! Provides [`VaultState`] — a Tauri-managed wrapper around
//! [`encryptman_keyring::Vault`] that stores the master key in the OS keychain
//! and delegates encryption/decryption to `encryptman` (AES-256-GCM + HKDF).

use encryptman_keyring::Vault;

/// Tauri-managed state wrapping an [`encryptman_keyring::Vault`].
///
/// The vault stores its master key in the OS native credential store:
/// - **Windows** — Credential Manager
/// - **macOS** — Keychain Services
/// - **Linux** — Secret Service (DBus)
pub struct VaultState(pub Vault);

//! Drug mapping engine (Phase 1).
//!
//! Links HOSxP drugs (`icode`) to INVS drugs (`working_code`) via the local
//! store.  Module layout:
//!
//! - [`normalizer`] — pure-Rust name normalization + similarity scoring
//!   (no I/O, unit-tested);
//! - [`repo`] — read/write operations on the local store;
//! - [`bulk`] — CSV parsing for bulk import (pure, unit-tested);
//! - [`commands`] — the Tauri IPC surface.

pub mod bulk;
pub mod commands;
pub mod normalizer;
pub mod repo;

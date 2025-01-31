//! Pluggable persistence.
//!
//! The persistence is a simple key-value store. The intention is to make it simple to implement
//! other persistence mechanisms than the provided ones, such as against a databases.

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::{
    collections::hash_map::{DefaultHasher, HashMap},
    fs,
    hash::{Hash, Hasher},
    io::Read,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use crate::{Error, Result};

/// Kinds of [persistence keys](struct.PersistKey.html).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum PersistKind {
    /// Persisted account private key.
    AccountPrivateKey,
    /// Persisted private key.
    PrivateKey,
    /// Persisted certificate.
    Certificate,
}

impl PersistKind {
    fn name(self) -> &'static str {
        match self {
            PersistKind::Certificate => "crt",
            PersistKind::PrivateKey => "key",
            PersistKind::AccountPrivateKey => "key",
        }
    }
}

/// Key for a value in the persistence.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct PersistKey<'a> {
    pub realm: u64,
    pub kind: PersistKind,
    pub key: &'a str,
}

impl<'a> PersistKey<'a> {
    /// Create a new key under a "realm", kind and key. The realm is an opaque hash
    /// of the given realm string.
    ///
    /// The realm is in normally defined as the account contact email, however it depends
    /// on how the `Account` object is accessed, see [`account_with_realm`].
    ///
    /// [`account_with_realm`]: ../struct.Directory.html#method.account_with_realm
    pub fn new(realm: &str, kind: PersistKind, key: &'a str) -> Self {
        let mut h = DefaultHasher::new();
        realm.hash(&mut h);
        let realm = h.finish();
        PersistKey { realm, kind, key }
    }

    pub(crate) fn path_in(&self, dir: &Path) -> PathBuf {
        let mut path = dir.join(self.to_string());
        path.set_extension(self.kind.name());
        path
    }
}

impl<'a> std::fmt::Display for PersistKey<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}_{}_{}",
            self.realm,
            self.kind.name(),
            self.key.replace('.', "_").replace('*', "STAR")
        )
    }
}

/// Trait for a persistence implementation.
///
/// Implementation must be clonable and thread safe (Send). This can easily be done by
/// wrapping the implemetation an `Arc<Mutex<P>>`.
pub trait Persist: Clone + Send {
    /// Store the given bytes under the given key.
    fn put(&self, key: &PersistKey, value: &[u8]) -> Result<()>;
    /// Read the bytes stored under the given key.
    ///
    /// `None` if the value doesn't exist.
    fn get(&self, key: &PersistKey) -> Result<Option<Vec<u8>>>;
}

/// Memory implementation for dev/testing.
///
/// The entries in memory are never saved to disk and are gone when the process dies.
///
/// Since the API is [rate limited] it's not a good idea to use this in production code.
///
/// [rate limited]: ../index.html#rate-limits
#[derive(Clone, Default)]
pub struct MemoryPersist {
    inner: Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

impl MemoryPersist {
    /// Create a memory persistence for testing.
    pub fn new() -> Self {
        MemoryPersist {
            ..Default::default()
        }
    }
}

impl Persist for MemoryPersist {
    fn put(&self, key: &PersistKey, value: &[u8]) -> Result<()> {
        let mut lock = self.inner.lock().unwrap();
        lock.insert(key.to_string(), value.to_owned());
        Ok(())
    }

    fn get(&self, key: &PersistKey) -> Result<Option<Vec<u8>>> {
        let lock = self.inner.lock().unwrap();
        Ok(lock.get(&key.to_string()).cloned())
    }
}

/// Simple file persistence.
///
/// Each key is saved under a unique filename.
#[derive(Clone)]
pub struct FilePersist {
    dir: PathBuf,
}

impl FilePersist {
    /// Create a file persistence in the directory pointed out by the `dir` given.
    ///
    /// The directory must be writable.
    pub fn new<P: AsRef<Path>>(dir: P) -> Self {
        FilePersist {
            dir: dir.as_ref().to_path_buf(),
        }
    }
}

impl Persist for FilePersist {
    #[cfg(not(unix))]
    fn put(&self, key: &PersistKey, value: &[u8]) -> Result<()> {
        fs::write(key.path_in(&self.dir), value).map_err(Error::from)
    }

    #[cfg(unix)]
    fn put(&self, key: &PersistKey, value: &[u8]) -> Result<()> {
        let path = key.path_from(&self.dir);
        match key.kind {
            PersistKind::AccountPrivateKey | PersistKind::PrivateKey => fs::OpenOptions::new()
                .mode(0o600)
                .write(true)
                .truncate(true)
                .create(true)
                .open(path)?
                .write_all(value)
                .map_err(Error::from),
            PersistKind::Certificate => fs::write(path, value).map_err(Error::from),
        }
    }

    fn get(&self, key: &PersistKey) -> Result<Option<Vec<u8>>> {
        let path = key.path_in(&self.dir);
        let ret = if let Ok(mut file) = fs::File::open(path) {
            let mut v = vec![];
            file.read_to_end(&mut v)?;
            Some(v)
        } else {
            None
        };
        Ok(ret)
    }
}

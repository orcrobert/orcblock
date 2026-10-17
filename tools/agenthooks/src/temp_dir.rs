use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

/// A directory under the system temp dir, removed on drop.
pub(crate) struct TempDir(PathBuf);

impl TempDir {
    pub(crate) fn new() -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("agenthooks-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create temp dir");
        Self(dir)
    }

    pub(crate) fn path(&self) -> &Path {
        &self.0
    }

    pub(crate) fn write(&self, rel: &str, content: &str) {
        let path = self.0.join(rel);
        fs::create_dir_all(path.parent().expect("joined path has a parent")).expect("mkdir");
        fs::write(path, content).expect("write");
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

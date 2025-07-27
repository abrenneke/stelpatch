use lasso::{Spur, ThreadedRodeo};

pub struct CaseInsensitiveInterner {
    inner: ThreadedRodeo,
}

/// An interner that stores strings in a case-insensitive manner.
impl CaseInsensitiveInterner {
    pub fn new() -> Self {
        Self {
            inner: ThreadedRodeo::new(),
        }
    }

    pub fn get_or_intern(&self, s: impl AsRef<str>) -> Spur {
        self.inner.get_or_intern(s.as_ref().to_lowercase())
    }

    pub fn resolve(&self, s: &Spur) -> &str {
        self.inner.resolve(s)
    }

    pub fn as_inner(&self) -> &ThreadedRodeo {
        &self.inner
    }

    pub fn into_inner(self) -> ThreadedRodeo {
        self.inner
    }
}

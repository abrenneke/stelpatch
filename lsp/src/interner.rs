use std::sync::OnceLock;

use lasso::ThreadedRodeo;

static INTERNER: OnceLock<ThreadedRodeo> = OnceLock::new();

/// Get the global interner
pub fn get_interner() -> &'static ThreadedRodeo {
    INTERNER.get_or_init(|| ThreadedRodeo::new())
}

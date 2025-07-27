use std::sync::OnceLock;

use cw_model::CaseInsensitiveInterner;

static INTERNER: OnceLock<CaseInsensitiveInterner> = OnceLock::new();

/// Get the global interner
pub fn get_interner() -> &'static CaseInsensitiveInterner {
    INTERNER.get_or_init(|| CaseInsensitiveInterner::new())
}

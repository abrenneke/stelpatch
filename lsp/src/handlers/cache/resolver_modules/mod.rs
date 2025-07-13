mod cache;
mod patterns;
mod properties;
mod references;
mod scopes;
mod subtypes;
mod utils;

pub use cache::TypeResolverCache;
pub use patterns::PatternMatcher;
pub use properties::PropertyNavigator;
pub use references::ReferenceResolver;
pub use scopes::ScopeHandler;
pub use subtypes::SubtypeHandler;
pub use utils::ResolverUtils;

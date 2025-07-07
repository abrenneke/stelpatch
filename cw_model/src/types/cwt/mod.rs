//! CWT (Clausewitz Type) Analysis Module
//!
//! This module provides tools for analyzing CWT (Clausewitz Type) files and converting
//! them to our rich InferredType system. It uses a visitor pattern with specialized
//! visitors for different CWT constructs.

pub mod analyzer;
pub mod conversion;
pub mod definitions;
pub mod options;
pub mod visitors;

pub use analyzer::*;
pub use conversion::*;
pub use definitions::*;
pub use options::*;
pub use visitors::*;

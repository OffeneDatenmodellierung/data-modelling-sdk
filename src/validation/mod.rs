//! Validation functionality
//! 
//! Provides validation logic for:
//! - Table validation (naming conflicts, pattern exclusivity)
//! - Relationship validation (circular dependencies)

pub mod tables;
pub mod relationships;

pub use tables::{TableValidationError, TableValidationResult};
pub use relationships::{RelationshipValidationError, RelationshipValidationResult};

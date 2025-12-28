//! Cross-domain reference models
//!
//! Defines structures for referencing tables and relationships from other domains.
//! This enables a domain to display and link to tables owned by other domains.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A reference to a table from another domain
/// 
/// This allows a domain to include tables from other domains in its canvas view.
/// The referenced table is read-only in the importing domain - it can only be
/// edited in its owning domain.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CrossDomainTableRef {
    /// Unique identifier for this reference
    pub id: Uuid,
    
    /// The domain that owns the table
    pub source_domain: String,
    
    /// The table ID in the source domain
    pub table_id: Uuid,
    
    /// Optional alias for display in this domain
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_alias: Option<String>,
    
    /// Position override for this domain's canvas (if different from source)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<super::Position>,
    
    /// Optional notes about why this table is referenced
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    
    /// When this reference was created
    #[serde(default = "chrono::Utc::now")]
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl CrossDomainTableRef {
    /// Create a new cross-domain table reference
    pub fn new(source_domain: String, table_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            source_domain,
            table_id,
            display_alias: None,
            position: None,
            notes: None,
            created_at: chrono::Utc::now(),
        }
    }
}

/// A reference to a relationship from another domain
/// 
/// When two tables from the same external domain are both imported,
/// their original relationship can be shown as a read-only link.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CrossDomainRelationshipRef {
    /// Unique identifier for this reference
    pub id: Uuid,
    
    /// The domain that owns the relationship
    pub source_domain: String,
    
    /// The relationship ID in the source domain
    pub relationship_id: Uuid,
    
    /// The source table ID (for quick lookup)
    pub source_table_id: Uuid,
    
    /// The target table ID (for quick lookup)
    pub target_table_id: Uuid,
    
    /// When this reference was created
    #[serde(default = "chrono::Utc::now")]
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl CrossDomainRelationshipRef {
    /// Create a new cross-domain relationship reference
    pub fn new(
        source_domain: String,
        relationship_id: Uuid,
        source_table_id: Uuid,
        target_table_id: Uuid,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            source_domain,
            relationship_id,
            source_table_id,
            target_table_id,
            created_at: chrono::Utc::now(),
        }
    }
}

/// The cross-domain configuration for a domain
/// 
/// This is stored as `cross_domain.yaml` in each domain's directory.
/// It defines which external tables and relationships should be visible
/// in this domain's canvas view.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct CrossDomainConfig {
    /// Schema version for forward compatibility
    #[serde(default = "default_schema_version")]
    pub schema_version: String,
    
    /// Tables imported from other domains
    #[serde(default)]
    pub imported_tables: Vec<CrossDomainTableRef>,
    
    /// Relationships imported from other domains (read-only display)
    /// These are automatically populated when both ends of a relationship
    /// from another domain are imported.
    #[serde(default)]
    pub imported_relationships: Vec<CrossDomainRelationshipRef>,
}

fn default_schema_version() -> String {
    "1.0".to_string()
}

impl CrossDomainConfig {
    /// Create a new empty cross-domain configuration
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Add a table reference from another domain
    /// Returns the index of the added or existing reference
    pub fn add_table_ref(&mut self, source_domain: String, table_id: Uuid) -> usize {
        // Check if already exists
        if let Some(idx) = self.imported_tables.iter().position(|t| 
            t.source_domain == source_domain && t.table_id == table_id
        ) {
            return idx;
        }
        
        let ref_entry = CrossDomainTableRef::new(source_domain, table_id);
        self.imported_tables.push(ref_entry);
        self.imported_tables.len() - 1
    }
    
    /// Get a table reference by index
    pub fn get_table_ref(&self, idx: usize) -> Option<&CrossDomainTableRef> {
        self.imported_tables.get(idx)
    }
    
    /// Remove a table reference
    pub fn remove_table_ref(&mut self, table_id: Uuid) -> bool {
        let initial_len = self.imported_tables.len();
        self.imported_tables.retain(|t| t.table_id != table_id);
        
        // Also remove any relationship refs that involve this table
        self.imported_relationships.retain(|r| 
            r.source_table_id != table_id && r.target_table_id != table_id
        );
        
        self.imported_tables.len() != initial_len
    }
    
    /// Add a relationship reference (for read-only display)
    /// Returns the index of the added or existing reference
    pub fn add_relationship_ref(
        &mut self,
        source_domain: String,
        relationship_id: Uuid,
        source_table_id: Uuid,
        target_table_id: Uuid,
    ) -> usize {
        // Check if already exists
        if let Some(idx) = self.imported_relationships.iter().position(|r| 
            r.source_domain == source_domain && r.relationship_id == relationship_id
        ) {
            return idx;
        }
        
        let ref_entry = CrossDomainRelationshipRef::new(
            source_domain,
            relationship_id,
            source_table_id,
            target_table_id,
        );
        self.imported_relationships.push(ref_entry);
        self.imported_relationships.len() - 1
    }
    
    /// Get a relationship reference by index
    pub fn get_relationship_ref(&self, idx: usize) -> Option<&CrossDomainRelationshipRef> {
        self.imported_relationships.get(idx)
    }
    
    /// Remove a relationship reference
    pub fn remove_relationship_ref(&mut self, relationship_id: Uuid) -> bool {
        let initial_len = self.imported_relationships.len();
        self.imported_relationships.retain(|r| r.relationship_id != relationship_id);
        self.imported_relationships.len() != initial_len
    }
    
    /// Get all imported table IDs from a specific domain
    pub fn get_tables_from_domain(&self, domain: &str) -> Vec<Uuid> {
        self.imported_tables
            .iter()
            .filter(|t| t.source_domain == domain)
            .map(|t| t.table_id)
            .collect()
    }
    
    /// Check if a table is imported from another domain
    pub fn is_table_imported(&self, table_id: Uuid) -> bool {
        self.imported_tables.iter().any(|t| t.table_id == table_id)
    }
    
    /// Get the source domain for an imported table
    pub fn get_table_source_domain(&self, table_id: Uuid) -> Option<&str> {
        self.imported_tables
            .iter()
            .find(|t| t.table_id == table_id)
            .map(|t| t.source_domain.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_add_table_ref() {
        let mut config = CrossDomainConfig::new();
        let table_id = Uuid::new_v4();
        
        config.add_table_ref("finance".to_string(), table_id);
        
        assert_eq!(config.imported_tables.len(), 1);
        assert_eq!(config.imported_tables[0].source_domain, "finance");
        assert_eq!(config.imported_tables[0].table_id, table_id);
    }
    
    #[test]
    fn test_duplicate_table_ref() {
        let mut config = CrossDomainConfig::new();
        let table_id = Uuid::new_v4();
        
        config.add_table_ref("finance".to_string(), table_id);
        config.add_table_ref("finance".to_string(), table_id);
        
        // Should not add duplicate
        assert_eq!(config.imported_tables.len(), 1);
    }
    
    #[test]
    fn test_remove_table_ref_removes_relationships() {
        let mut config = CrossDomainConfig::new();
        let table_a = Uuid::new_v4();
        let table_b = Uuid::new_v4();
        let rel_id = Uuid::new_v4();
        
        config.add_table_ref("finance".to_string(), table_a);
        config.add_table_ref("finance".to_string(), table_b);
        config.add_relationship_ref("finance".to_string(), rel_id, table_a, table_b);
        
        assert_eq!(config.imported_relationships.len(), 1);
        
        config.remove_table_ref(table_a);
        
        // Relationship should also be removed
        assert_eq!(config.imported_relationships.len(), 0);
    }
}


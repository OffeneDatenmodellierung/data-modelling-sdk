//! Relationship validation functionality
//!
//! Validates relationships for circular dependencies, self-references, etc.
//!
//! This module implements SDK-native validation against SDK models.

use crate::models::Relationship;
use anyhow::Result;
use petgraph::{Directed, Graph};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Result of relationship validation.
///
/// Contains any circular dependencies or self-references found during validation.
#[derive(Debug, Serialize, Deserialize)]
#[must_use = "validation results should be checked for circular dependencies and self-references"]
pub struct RelationshipValidationResult {
    /// Circular dependencies found
    pub circular_dependencies: Vec<CircularDependency>,
    /// Self-references found
    pub self_references: Vec<SelfReference>,
}

/// Circular dependency detected
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircularDependency {
    pub relationship_id: Uuid,
    pub cycle_path: Vec<Uuid>,
}

/// Self-reference detected
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfReference {
    pub relationship_id: Uuid,
    pub table_id: Uuid,
}

/// Error during relationship validation
#[derive(Debug, thiserror::Error, Serialize, Deserialize)]
pub enum RelationshipValidationError {
    #[error("Validation error: {0}")]
    ValidationError(String),
}

/// Relationship validator
#[derive(Default)]
pub struct RelationshipValidator;

impl RelationshipValidator {
    /// Create a new relationship validator
    ///
    /// # Example
    ///
    /// ```rust
    /// use data_modelling_sdk::validation::relationships::RelationshipValidator;
    ///
    /// let validator = RelationshipValidator::new();
    /// ```
    pub fn new() -> Self {
        Self
    }

    /// Check for circular dependencies using graph cycle detection
    ///
    /// Uses petgraph to detect cycles in the relationship graph. If adding the new relationship
    /// would create a cycle, returns the cycle path.
    ///
    /// # Arguments
    ///
    /// * `relationships` - Existing relationships in the model
    /// * `source_table_id` - Source table ID of the new relationship
    /// * `target_table_id` - Target table ID of the new relationship
    ///
    /// # Returns
    ///
    /// A tuple of (has_cycle: bool, cycle_path: Option<Vec<Uuid>>).
    /// If a cycle is detected, the cycle path contains the table IDs forming the cycle.
    ///
    /// # Example
    ///
    /// ```rust
    /// use data_modelling_sdk::validation::relationships::RelationshipValidator;
    /// use data_modelling_sdk::models::Relationship;
    ///
    /// let validator = RelationshipValidator::new();
    /// let table_a = uuid::Uuid::new_v4();
    /// let table_b = uuid::Uuid::new_v4();
    /// let table_c = uuid::Uuid::new_v4();
    ///
    /// // Create a cycle: A -> B -> C -> A
    /// let rels = vec![
    ///     Relationship::new(table_a, table_b),
    ///     Relationship::new(table_b, table_c),
    /// ];
    ///
    /// let (has_cycle, _) = validator.check_circular_dependency(&rels, table_c, table_a).unwrap();
    /// assert!(has_cycle);
    /// ```
    pub fn check_circular_dependency(
        &self,
        relationships: &[Relationship],
        source_table_id: Uuid,
        target_table_id: Uuid,
    ) -> Result<(bool, Option<Vec<Uuid>>), RelationshipValidationError> {
        // Build a directed graph from relationships
        let mut graph = Graph::<Uuid, Uuid, Directed>::new();
        let mut node_map = std::collections::HashMap::new();

        // Add all tables as nodes
        for rel in relationships {
            let source_node = *node_map
                .entry(rel.source_table_id)
                .or_insert_with(|| graph.add_node(rel.source_table_id));
            let target_node = *node_map
                .entry(rel.target_table_id)
                .or_insert_with(|| graph.add_node(rel.target_table_id));
            graph.add_edge(source_node, target_node, rel.id);
        }

        // Add the new relationship being checked
        let source_node = *node_map
            .entry(source_table_id)
            .or_insert_with(|| graph.add_node(source_table_id));
        let target_node = *node_map
            .entry(target_table_id)
            .or_insert_with(|| graph.add_node(target_table_id));
        // Synthetic edge id (only used internally in the graph).
        let edge_id = Uuid::new_v4();
        graph.add_edge(source_node, target_node, edge_id);

        // Check for cycles using simple reachability check
        // Note: find_negative_cycle requires FloatMeasure trait, so we use a simpler approach
        // Check if target can reach source (which would create a cycle)
        if self.can_reach(&graph, &node_map, target_table_id, source_table_id) {
            // Build cycle path
            let cycle_path = self
                .find_path(&graph, &node_map, target_table_id, source_table_id)
                .unwrap_or_default();
            return Ok((true, Some(cycle_path)));
        }

        Ok((false, None))
    }

    /// Check if target can reach source in the graph
    fn can_reach(
        &self,
        graph: &Graph<Uuid, Uuid, Directed>,
        node_map: &std::collections::HashMap<Uuid, petgraph::graph::NodeIndex>,
        from: Uuid,
        to: Uuid,
    ) -> bool {
        if let (Some(&from_idx), Some(&to_idx)) = (node_map.get(&from), node_map.get(&to)) {
            // Use DFS to check reachability
            let mut visited = std::collections::HashSet::new();
            let mut stack = vec![from_idx];

            while let Some(node) = stack.pop() {
                if node == to_idx {
                    return true;
                }
                if visited.insert(node) {
                    for neighbor in graph.neighbors(node) {
                        if !visited.contains(&neighbor) {
                            stack.push(neighbor);
                        }
                    }
                }
            }
        }
        false
    }

    /// Find a path from source to target
    fn find_path(
        &self,
        graph: &Graph<Uuid, Uuid, Directed>,
        node_map: &std::collections::HashMap<Uuid, petgraph::graph::NodeIndex>,
        from: Uuid,
        to: Uuid,
    ) -> Option<Vec<Uuid>> {
        if let (Some(&from_idx), Some(&to_idx)) = (node_map.get(&from), node_map.get(&to)) {
            // Use BFS to find path
            let mut visited = std::collections::HashSet::new();
            let mut queue = std::collections::VecDeque::new();
            let mut parent = std::collections::HashMap::new();

            queue.push_back(from_idx);
            visited.insert(from_idx);

            while let Some(node) = queue.pop_front() {
                if node == to_idx {
                    // Reconstruct path
                    let mut path = Vec::new();
                    let mut current = Some(to_idx);
                    while let Some(node_idx) = current {
                        path.push(graph[node_idx]);
                        current = parent.get(&node_idx).copied();
                    }
                    path.reverse();
                    return Some(path);
                }

                for neighbor in graph.neighbors(node) {
                    if !visited.contains(&neighbor) {
                        visited.insert(neighbor);
                        parent.insert(neighbor, node);
                        queue.push_back(neighbor);
                    }
                }
            }
        }
        None
    }

    /// Validate that source and target tables are different
    pub fn validate_no_self_reference(
        &self,
        source_table_id: Uuid,
        target_table_id: Uuid,
    ) -> Result<(), SelfReference> {
        if source_table_id == target_table_id {
            return Err(SelfReference {
                relationship_id: Uuid::new_v4(),
                table_id: source_table_id,
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_self_reference() {
        let id = Uuid::new_v4();
        let err = RelationshipValidator::new()
            .validate_no_self_reference(id, id)
            .unwrap_err();
        assert_eq!(err.table_id, id);
    }

    #[test]
    fn detects_cycle() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let rel1 = Relationship::new(a, b);
        let validator = RelationshipValidator::new();
        // adding b->a would create a cycle
        let (has_cycle, path) = validator.check_circular_dependency(&[rel1], b, a).unwrap();
        assert!(has_cycle);
        assert!(path.is_some());
    }
}

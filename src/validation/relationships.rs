//! Relationship validation functionality
//! 
//! Validates relationships for circular dependencies, self-references, etc.
//! 
//! This module wraps validation logic from the parent crate.
//! In a full migration, the validation logic would be moved here.

use anyhow::Result;
use petgraph::{Graph, Directed};
use uuid::Uuid;

/// Result of relationship validation
#[derive(Debug)]
pub struct RelationshipValidationResult {
    /// Circular dependencies found
    pub circular_dependencies: Vec<CircularDependency>,
    /// Self-references found
    pub self_references: Vec<SelfReference>,
}

/// Circular dependency detected
#[derive(Debug, Clone)]
pub struct CircularDependency {
    pub relationship_id: Uuid,
    pub cycle_path: Vec<Uuid>,
}

/// Self-reference detected
#[derive(Debug, Clone)]
pub struct SelfReference {
    pub relationship_id: Uuid,
    pub table_id: Uuid,
}

/// Error during relationship validation
#[derive(Debug, thiserror::Error)]
pub enum RelationshipValidationError {
    #[error("Validation error: {0}")]
    ValidationError(String),
}

/// Relationship validator
pub struct RelationshipValidator;

impl RelationshipValidator {
    /// Create a new relationship validator
    pub fn new() -> Self {
        Self
    }

    /// Check for circular dependencies using graph cycle detection
    /// 
    /// This wraps RelationshipService::check_circular_dependency from the parent crate.
    /// In a full migration, the logic would be moved here.
    /// 
    /// Uses petgraph to detect cycles in the relationship graph.
    pub fn check_circular_dependency(
        &self,
        relationships: &[RelationshipData],
        source_table_id: Uuid,
        target_table_id: Uuid,
    ) -> Result<(bool, Option<Vec<Uuid>>), RelationshipValidationError> {
        // Build a directed graph from relationships
        let mut graph = Graph::<Uuid, Uuid, Directed>::new();
        let mut node_map = std::collections::HashMap::new();
        
        // Add all tables as nodes
        for rel in relationships {
            let source_node = *node_map.entry(rel.source_table_id)
                .or_insert_with(|| graph.add_node(rel.source_table_id));
            let target_node = *node_map.entry(rel.target_table_id)
                .or_insert_with(|| graph.add_node(rel.target_table_id));
            graph.add_edge(source_node, target_node, rel.id);
        }
        
        // Add the new relationship being checked
        let source_node = *node_map.entry(source_table_id)
            .or_insert_with(|| graph.add_node(source_table_id));
        let target_node = *node_map.entry(target_table_id)
            .or_insert_with(|| graph.add_node(target_table_id));
        // Use deterministic UUID for edge (based on source and target table IDs)
        let edge_id = crate::models::relationship::Relationship::generate_id(source_table_id, target_table_id);
        graph.add_edge(source_node, target_node, edge_id);
        
        // Check for cycles using simple reachability check
        // Note: find_negative_cycle requires FloatMeasure trait, so we use a simpler approach
        // Check if target can reach source (which would create a cycle)
        if self.can_reach(&graph, &node_map, target_table_id, source_table_id) {
            // Build cycle path
            let cycle_path = self.find_path(&graph, &node_map, target_table_id, source_table_id)
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
            // Generate deterministic UUID for error reporting
            let error_id = crate::models::relationship::Relationship::generate_id(source_table_id, target_table_id);
            return Err(SelfReference {
                relationship_id: error_id,
                table_id: source_table_id,
            });
        }
        Ok(())
    }
}

// Placeholder types
#[derive(Debug, Clone)]
pub struct RelationshipData {
    pub id: Uuid,
    pub source_table_id: Uuid,
    pub target_table_id: Uuid,
}

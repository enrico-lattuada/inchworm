//! Topological sort of name-dependency graph using Kahn's algorithm.
//!
//! Used to resolve forward references when loading a batch of named definitions at once.
//! Unresolved names are treated as already satisfied. Resolving that is deferred to whoever
//! actually looks the name up later.

use std::collections::{BTreeSet, HashMap, HashSet};

use crate::DimensionError;

/// Returns the names in the graph in topological order.
///
/// Each entry in the graph represents the dependencies of a name from its definition.
/// Dependencies which are not a key in the graph are treated as already resolved.
///
/// The order is deterministic: at each step, the alphabetically first name whose
/// dependencies are met comes next.
///
/// # Errors
///
/// Returns [`DimensionError::CyclicDefinition`] if cycles in the definitions are detected.
/// The error will contain the names involved in the cycle, including those blocked by the cycle,
/// alphabetically sorted.
pub(crate) fn topological_order(
    graph: &HashMap<String, Vec<String>>,
) -> Result<Vec<String>, DimensionError> {
    // Deduplicate the references in the dependencies, so they each count as 1
    let deduped_graph: HashMap<String, HashSet<String>> = graph
        .iter()
        .map(|(k, v)| (k.clone(), v.iter().cloned().collect()))
        .collect();
    let mut in_degree_count: HashMap<String, usize> = deduped_graph
        .iter()
        .map(|(k, v)| {
            let count = v.iter().filter(|&ident| graph.contains_key(ident)).count();
            (k.clone(), count)
        })
        .collect();
    // External names don't get edges
    let mut reverse_graph: HashMap<String, HashSet<String>> = HashMap::new();
    for (k, deps) in deduped_graph {
        for dep in deps.iter().filter(|&ident| graph.contains_key(ident)) {
            reverse_graph
                .entry(dep.clone())
                .or_default()
                .insert(k.clone());
        }
    }
    let mut queue: BTreeSet<String> = in_degree_count
        .iter()
        .filter(|(_, count)| **count == 0)
        .map(|(k, _)| k.clone())
        .collect();
    let mut output = Vec::new();
    while let Some(name) = queue.pop_first() {
        output.push(name.clone());
        for dep in reverse_graph.get(&name).into_iter().flatten() {
            in_degree_count.entry(dep.clone()).and_modify(|count| {
                *count -= 1;
                if *count == 0 {
                    queue.insert(dep.clone());
                }
            });
        }
    }
    // Anything left with a nonzero count means something it depends on never resolved
    if output.len() != graph.len() {
        let mut cyclic_def_names: Vec<String> = in_degree_count
            .iter()
            .filter(|(_, count)| **count > 0)
            .map(|(name, _)| name.clone())
            .collect();
        cyclic_def_names.sort_unstable();
        return Err(DimensionError::CyclicDefinition {
            names: cyclic_def_names,
        });
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::errors_match;

    mod topological_order {
        use super::*;

        #[test]
        fn empty_graph_gives_empty_order() {
            let graph = HashMap::new();
            let order = topological_order(&graph).unwrap();
            let expected_order: Vec<String> = Vec::new();
            assert!(
                order == expected_order,
                "expected empty order, got {order:?}"
            );
        }

        #[test]
        fn single_node_with_no_deps_returns_itself() {
            let mut graph = HashMap::new();
            graph.insert("A".into(), Vec::new());
            let order = topological_order(&graph).unwrap();
            let expected_order: Vec<String> = vec!["A".into()];
            assert!(order == expected_order);
        }

        #[test]
        fn orders_scrambled_chain() {
            let mut graph: HashMap<String, Vec<String>> = HashMap::new();
            graph.insert("C".into(), vec!["B".into()]);
            graph.insert("B".into(), vec!["A".into()]);
            graph.insert("A".into(), Vec::new());
            let order = topological_order(&graph).unwrap();
            assert_eq!(order, ["A", "B", "C"]);
        }

        #[test]
        fn handles_diamond_dependency() {
            let mut graph: HashMap<String, Vec<String>> = HashMap::new();
            graph.insert("A".into(), Vec::new());
            graph.insert("B".into(), vec!["A".into()]);
            graph.insert("C".into(), vec!["A".into()]);
            graph.insert("D".into(), vec!["B".into(), "C".into()]);
            let order = topological_order(&graph).unwrap();
            assert_eq!(order, ["A", "B", "C", "D"]);
        }

        #[test]
        fn orders_ready_names_alphabetically() {
            let mut graph: HashMap<String, Vec<String>> = HashMap::new();
            graph.insert("c".into(), Vec::new());
            graph.insert("a".into(), Vec::new());
            graph.insert("b".into(), Vec::new());
            let order = topological_order(&graph).unwrap();
            assert_eq!(order, ["a", "b", "c"]);
        }

        #[test]
        fn ignores_duplicate_reference_within_entry() {
            let mut graph: HashMap<String, Vec<String>> = HashMap::new();
            graph.insert("A".into(), Vec::new());
            graph.insert("B".into(), vec!["A".into(), "A".into()]);
            assert!(
                topological_order(&graph).is_ok(),
                "topological order must ignore duplicate dependencies"
            );
        }

        #[test]
        fn detects_direct_cycle() {
            let mut graph: HashMap<String, Vec<String>> = HashMap::new();
            graph.insert("B".into(), vec!["A".into()]);
            graph.insert("A".into(), vec!["B".into()]);
            let err = topological_order(&graph);
            let expected = DimensionError::CyclicDefinition {
                names: vec!["A".into(), "B".into()],
            };
            assert!(errors_match(&err.unwrap_err(), &expected));
        }

        #[test]
        fn detects_cycle_blocks_downstream_dependent() {
            let mut graph: HashMap<String, Vec<String>> = HashMap::new();
            graph.insert("B".into(), vec!["A".into()]);
            graph.insert("A".into(), vec!["B".into()]);
            graph.insert("C".into(), vec!["A".into()]);
            let err = topological_order(&graph);
            let expected = DimensionError::CyclicDefinition {
                names: vec!["A".into(), "B".into(), "C".into()],
            };
            assert!(errors_match(&err.unwrap_err(), &expected));
        }

        #[test]
        fn treats_unresolved_name_as_already_satisfied() {
            let mut graph: HashMap<String, Vec<String>> = HashMap::new();
            graph.insert("B".into(), vec!["A".into()]);
            let order = topological_order(&graph);
            assert!(
                order.is_ok(),
                "topological order with unresolved names should succeed"
            );
        }
    }
}

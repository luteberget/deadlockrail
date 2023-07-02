use log::{info, warn, debug};

use crate::{plan::DeadlockResult, problem::Problem};
use std::collections::{BTreeMap, BTreeSet, HashSet};
type Occ<'a> = Vec<&'a String>;

pub fn solve(problem: &Problem) -> DeadlockResult {
    let init_string = "init".to_string();
    let initnode = vec![&init_string];

    let p_init = hprof::enter("cycle init");
    // let inits = problem
    //     .trains
    //     .iter()
    //     .map(sorted_initial_routes)
    //     .collect::<Vec<_>>();

    let movement_graphs = problem
        .trains
        .iter()
        .map(|t| movement_graph(t, initnode.clone()))
        .collect::<Vec<_>>();

    let mut s = idl::IdlSolver::new();

    struct TrainVars<'a> {
        node: BTreeMap<&'a Occ<'a>, (idl::Lit, idl::DVar)>,
        edge: BTreeMap<(&'a Occ<'a>, &'a Occ<'a>), idl::Lit>,
    }

    let mut train_vars = Vec::new();

    // Need to choose a path through the graph
    for train_idx in 0..problem.trains.len() {
        // let init_node = &inits[train_idx];
        let movement_graph = &movement_graphs[train_idx];
        let mut node_vars: BTreeMap<&Occ, (idl::Lit, idl::DVar)> = Default::default();

        let init_node_var = s.new_bool();

        let mut train_edges: BTreeMap<(&Occ, &Occ), idl::Lit> = Default::default();
        node_vars.insert(&initnode, (init_node_var, s.new_int()));
        let mut unvisited = vec![&initnode];

        // DFS through the movement graph and add variables and flow constraints.
        while let Some(state) = unvisited.pop() {
            // println!(
            //     "TRAIN {} state {:?} edges {:?}",
            //     train_idx, state, movement_graph[state]
            // );

            let is_black_hole = movement_graph[state].is_empty();
            assert!(state.is_empty() == is_black_hole);

            if is_black_hole {
                // No constraints on the black hole node.

                continue;
            }

            let (var, source_time) = node_vars[&state];
            let next_vars = movement_graph[state]
                .iter()
                .map(|(next, _)| {
                    (
                        next,
                        *node_vars.entry(next).or_insert_with(|| {
                            unvisited.push(next);
                            (s.new_bool(), s.new_int())
                        }),
                    )
                })
                .collect::<Vec<_>>();

            let edge_vars = next_vars
                .iter()
                .map(|(next, (target_active, target_time))| {
                    let var = s.new_bool();
                    train_edges.insert((state, *next), var);
                    (var, *target_active, *target_time)
                })
                .collect::<Vec<_>>();

            // Choose at least one edge if node is active.
            let mut clause = edge_vars
                .iter()
                .map(|(edge_active, _, _)| *edge_active)
                .collect::<Vec<_>>();
            clause.push(!var);
            s.add_clause(&clause);

            // // Choose at most one edge.
            // for e1 in 0..edge_vars.len() {
            //     for e2 in (e1 + 1)..edge_vars.len() {
            //         s.add_clause(&vec![!edge_vars[e1].0, !edge_vars[e2].0]);
            //     }
            // }

            // Add difference constraint and target node activation
            for (edge_active, target_active, target_time) in edge_vars.iter().copied() {
                // println!(
                //     "Add diff {:?} {:?} {:?}",
                //     edge_active, source_time, target_time
                // );
                s.add_diff(Some(edge_active), source_time, target_time, -1);

                s.add_clause(&vec![!edge_active, target_active]);
            }
        }

        // Each node in the movement graph should now have an associated bool variable.
        assert!(movement_graph.len() == node_vars.len());
        train_vars.push(TrainVars {
            node: node_vars,
            edge: train_edges,
        })
    }

    let mut n_conflicts = 0;
    // For all pairs of movement nodes from different trains, deconflict.
    for t1_idx in 0..problem.trains.len() {
        let t1edges = movement_graphs[t1_idx]
            .iter()
            .flat_map(|(n1, es)| es.iter().map(move |(n2, temp)| (n1, n2, temp)));

        for (t1n1, t1n2, t1temp) in t1edges {
            let mut t1_resources: HashSet<&String> = Default::default();
            for x in [t1n1, t1n2, t1temp] {
                t1_resources.extend(x.iter().filter(|r| **r != "init").flat_map(|r| {
                    problem.trains[t1_idx].routes[*r]
                        .unconditional_conflicts
                        .iter()
                        .chain(std::iter::once(*r))
                }));
            }

            for t2_idx in (t1_idx + 1)..problem.trains.len() {
                let t2edges = movement_graphs[t2_idx]
                    .iter()
                    .flat_map(|(n1, es)| es.iter().map(move |(n2, temp)| (n1, n2, temp)));

                for (t2n1, t2n2, t2temp) in t2edges {
                    let in_conflict = [t2n1, t2n2, t2temp]
                        .iter()
                        .any(|routes| routes.iter().any(|r| t1_resources.contains(r)));

                    if in_conflict {
                        let t1_is_initial = t1n1 == &initnode;
                        let t2_is_initial = t2n1 == &initnode;
                        if t1_is_initial && t2_is_initial {
                            warn!("Ignoring conflict in initial state {} {:?} {:?} {:?} {} {:?} {:?} {:?}",
                            t1_idx, t1n1, t1temp, t1n2, t2_idx, t2n1, t2temp, t2n2);
                            warn!("  {:?}", t1_resources);
                        } else if t1_is_initial {
                            let c = s.new_bool();
                            debug!(" C {:?}:   {:?} < {:?}", c, t1n2, t2n1);

                            s.add_diff(
                                Some(c),
                                train_vars[t1_idx].node[t1n2].1,
                                train_vars[t2_idx].node[t2n1].1,
                                -1,
                            );
                            s.add_clause(&vec![
                                !train_vars[t1_idx].edge[&(t1n1, t1n2)],
                                !train_vars[t2_idx].edge[&(t2n1, t2n2)],
                                c,
                            ]);
                        } else if t2_is_initial {
                            let c = s.new_bool();
                            debug!(" C {:?}:   {:?} < {:?}", c, t2n2, t1n1);

                            s.add_diff(
                                Some(c),
                                train_vars[t2_idx].node[t2n2].1,
                                train_vars[t1_idx].node[t1n1].1,
                                -1,
                            );
                            s.add_clause(&vec![
                                !train_vars[t1_idx].edge[&(t1n1, t1n2)],
                                !train_vars[t2_idx].edge[&(t2n1, t2n2)],
                                c,
                            ]);
                        } else {
                            let c1 = s.new_bool();
                            let c2 = s.new_bool();
                            debug!(
                                "Adding conflict {} {:?} {:?} {:?} {} {:?} {:?} {:?}",
                                t1_idx, t1n1, t1temp, t1n2, t2_idx, t2n1, t2temp, t2n2
                            );

                            debug!(" C1 {:?}:   {:?} < {:?}", c1, t2n2, t1n1);
                            debug!(" C2 {:?}:   {:?} < {:?}", c2, t1n2, t2n1);

                            s.add_diff(
                                Some(c1),
                                train_vars[t2_idx].node[t2n2].1,
                                train_vars[t1_idx].node[t1n1].1,
                                -1,
                            );

                            s.add_diff(
                                Some(c2),
                                train_vars[t1_idx].node[t1n2].1,
                                train_vars[t2_idx].node[t2n1].1,
                                -1,
                            );

                            s.add_clause(&vec![
                                !train_vars[t1_idx].edge[&(t1n1, t1n2)],
                                !train_vars[t2_idx].edge[&(t2n1, t2n2)],
                                c1,
                                c2,
                            ]);

                            n_conflicts += 1;
                        }
                    }
                }
            }
        }
    }

    // 2023-07-03 There is a bug in the IDL solver, which doesn't handle
    // any literals being set before adding the difference constraints,
    // which happens when unit clauses are added, interleaved with the
    // conditional difference constraints.
    // Here, we have postponed adding the unit clauses until now, pending
    // a bug fix in the IDL library.

    for train_idx in 0..problem.trains.len() {
        s.add_clause(&vec![train_vars[train_idx].node[&initnode].0]);
    }

    println!("n_conflicts={}", n_conflicts);

    drop(p_init);
    let _p = hprof::enter("cycle solve");

    match s.solve() {
        Ok(_model) => {
            info!("LIVE");
            for (train_idx, trainvar) in train_vars.iter().enumerate() {
                for (node, (on, t)) in trainvar.node.iter() {
                    println!(
                        "train {} node {:?} on={:?}=={} t={:?}=={}",
                        train_idx,
                        node,
                        on,
                        s.get_bool_value(*on),
                        t,
                        s.get_int_value(*t)
                    );
                }
                for ((n1, n2), on) in trainvar.edge.iter() {
                    println!(
                        "train {} EDGE {:?}--->{:?} on={:?}=={}",
                        train_idx,
                        n1,
                        n2,
                        on,
                        s.get_bool_value(*on)
                    );
                }
            }

            DeadlockResult::Live(crate::plan::Plan {
                steps: vec![ /* TODO */],
            })
        }
        Err(_) => {
            info!("DEADLOCKED");
            DeadlockResult::Deadlocked(())
        }
    }
}

fn movement_graph<'a>(
    train: &'a crate::problem::Train,
    dummy_init: Vec<&'a String>,
) -> BTreeMap<Vec<&'a String>, BTreeSet<(Vec<&'a String>, Vec<&'a String>)>> {
    // In Sasso 2023 bencmarks, the original list of initial routes is not consistently ordered.
    assert!(!train.initial_routes.is_empty());
    let init_routes = sorted_initial_routes(train);

    let mut graph: BTreeMap<Occ, BTreeSet<(Occ, Occ)>> = Default::default();
    graph
        .entry(dummy_init)
        .or_default()
        .insert((init_routes.clone(), vec![]));
    graph.insert(init_routes.clone(), Default::default());

    let mut states = vec![init_routes];
    while let Some(state) = states.pop() {
        // println!("VISITING {:?}", state);
        let head = state.last().unwrap();
        if let Some(nexts) = train.routes[*head].next_routes.as_ref() {
            for next in nexts.iter() {
                // Now, make a new state.
                let mut to_drop = state.clone();
                to_drop.push(next);

                // Find the routes that can eventually be dropped when
                // the new route has been reached.

                let mut split_idx = to_drop.len();
                let mut remaining_length = train.routes[next].train_length;
                while split_idx >= 1 && remaining_length > 0 {
                    split_idx -= 1;
                    let route_length = train.routes[to_drop[split_idx]].route_length;
                    remaining_length = remaining_length.saturating_sub(route_length);
                }

                let new_state = to_drop.split_off(split_idx.min(to_drop.len() - 1));

                // println!("state {:?} -> drop {:?} -> next {:?}", state, to_drop, new_state);

                graph
                    .get_mut(&state)
                    .unwrap()
                    .insert((new_state.clone(), to_drop));

                graph.entry(new_state.clone()).or_insert_with(|| {
                    states.push(new_state);
                    Default::default()
                });
            }
        } else {
            // Black hole, so link up to the empty set.
            let black_hole = vec![];
            graph.entry(black_hole.clone()).or_default();
            graph
                .entry(state.clone())
                .or_default()
                .insert((black_hole, state));
        }
    }

    // Should have reached a black hole.
    assert!(graph.contains_key(&vec![]));
    // Only black holes should be dead-ends.
    assert!(graph.iter().all(|(k, v)| k.is_empty() == v.is_empty()));
    graph
}

fn sorted_initial_routes(train: &crate::problem::Train) -> Vec<&String> {
    // Let's assume no trains are headed for a black hole in the initial state.
    assert!(train
        .initial_routes
        .iter()
        .all(|n| train.routes[n].next_routes.is_some()));

    // Check that exactly one of the initial routes does not point to one of the other
    // initial routes.
    assert!(
        train
            .initial_routes
            .iter()
            .filter(|n| !train.routes[*n]
                .next_routes
                .as_ref()
                .unwrap()
                .iter()
                .any(|r| train.initial_routes.contains(r)))
            .count()
            == 1
    );

    // Insert in sorted order
    let mut rs = Vec::new();
    // Insert the last one first
    rs.push(
        train
            .initial_routes
            .iter()
            .find(|n| {
                !train.routes[*n]
                    .next_routes
                    .as_ref()
                    .unwrap()
                    .iter()
                    .any(|r| train.initial_routes.contains(r))
            })
            .unwrap(),
    );

    // Insert the one poining to the previous.
    while rs.len() < train.initial_routes.len() {
        rs.push(
            train
                .initial_routes
                .iter()
                .find(|n| {
                    train.routes[*n]
                        .next_routes
                        .as_ref()
                        .unwrap()
                        .iter()
                        .any(|r| r == *rs.last().unwrap())
                })
                .unwrap(),
        );
    }

    rs.reverse();
    rs
}

#[allow(unused)]
use log::{debug, info, warn};

use crate::{plan::DeadlockResult, problem::Problem};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
type Occ<'a> = Vec<&'a String>;

pub fn solve(problem: &Problem) -> DeadlockResult {
    let empty_node: Vec<&String> = vec![];
    let p_init = hprof::enter("cycle init");
    let mut movement_graphs = Vec::new();
    let mut route_allocation_blocked_by: HashMap<&String, HashSet<(usize, Vec<&String>)>> =
        Default::default();

    let mut train_init_routes = Vec::new();

    for (train_idx, train) in problem.trains.iter().enumerate() {
        let g = {
            // In Sasso 2023 bencmarks, the original list of initial routes is not consistently ordered.
            assert!(!train.initial_routes.is_empty());
            let init_routes = sorted_initial_routes(train);
            train_init_routes.push(init_routes.clone());

            let mut graph: BTreeMap<Occ, BTreeSet<(Occ, Occ)>> = Default::default();
            graph.insert(init_routes.clone(), Default::default());

            let mut states = vec![init_routes];
            while let Some(state) = states.pop() {
                // Make a look-up structure of which routes cannot be allocated when this train is in this node.
                let entering = state.last().unwrap();
                println!("STATE {:?} entering {:?}", state, entering);
                let mut remaining_length = train.routes[*entering].train_length;
                for route in state.iter().copied().rev() {
                    let r = &train.routes[route];
                    for unconditional_conflict in r
                        .unconditional_conflicts
                        .iter()
                        .chain(std::iter::once(*entering))
                    {
                        println!(
                            "{} blocked by {:?}",
                            unconditional_conflict,
                            (train_idx, state.clone())
                        );
                        route_allocation_blocked_by
                            .entry(unconditional_conflict)
                            .or_default()
                            .insert((train_idx, state.clone()));
                    }

                    remaining_length =
                        remaining_length.saturating_sub(r.route_length_without_switch);
                    if remaining_length == 0 {
                        break;
                    }

                    for allocation_conflict in train.routes[route].allocation_conflicts.iter() {
                        println!(
                            "{} blocked by {:?}",
                            allocation_conflict,
                            (train_idx, state.clone())
                        );
                        route_allocation_blocked_by
                            .entry(allocation_conflict)
                            .or_default()
                            .insert((train_idx, state.clone()));
                    }

                    remaining_length = remaining_length
                        .saturating_sub(r.route_length - r.route_length_without_switch);
                    if remaining_length == 0 {
                        break;
                    }
                }

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
        };
        movement_graphs.push(g);
    }

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

        let mut train_edges: BTreeMap<(&Occ, &Occ), idl::Lit> = Default::default();
        node_vars.insert(&train_init_routes[train_idx], (s.new_bool(), s.new_int()));
        let mut unvisited = vec![&train_init_routes[train_idx]];

        // DFS through the movement graph and add variables and flow constraints.
        while let Some(state) = unvisited.pop() {
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

    fn advance_until_stops_blocking<'a>(
        blocked_by: &HashSet<(usize, Vec<&String>)>,
        movement_graph: &BTreeMap<Vec<&'a String>, BTreeSet<(Vec<&'a String>, Vec<&'a String>)>>,
        t2: &usize,
        t2_node: Vec<&'a String>,
    ) -> Vec<Vec<&'a String>> {
        let mut alternatives: HashSet<Vec<&String>> = Default::default();
        // Here, we look at permanent conflicts.

        let mut queue = VecDeque::new();
        queue.push_back(t2_node);

        while let Some(t2_node) = queue.pop_front() {
            if !blocked_by.contains(&(*t2, t2_node.clone())) {
                alternatives.insert(t2_node);
            } else {
                let nexts = &movement_graph[&t2_node];
                assert!(!nexts.is_empty());
                queue.extend(nexts.iter().map(|(n, _)| n.clone()));
            }
        }

        alternatives.into_iter().collect()
    }

    let mut n_conflicts = 0;
    // For all pairs of movement nodes from different trains, deconflict.
    for t1_idx in 0..problem.trains.len() {
        for t1_node in movement_graphs[t1_idx].keys() {
            if t1_node == &empty_node {
                continue;
            }

            let t1_entering_route = t1_node.last().unwrap();

            // Which blocking sets in other trains are blocking me from entering this route?
            if let Some(blocks) = route_allocation_blocked_by.get(t1_entering_route) {
                assert!(t1_node != &empty_node || blocks.is_empty());

                for (t2_idx, t2_node) in blocks {
                    assert!(t2_node != &empty_node);

                    if *t2_idx == t1_idx {
                        continue;
                    }

                    let t2_entering_route = t2_node.last().unwrap();
                    println!(
                        "train {}  t2_node {:?} t2_entering {:?}",
                        t2_idx, t2_node, t2_entering_route
                    );

                    // Allocation is blocked by t2_node.
                    // Either t2 needs to stop blocking t1_entering_route before
                    // we reach this node.

                    let t2_first_alternatives = advance_until_stops_blocking(
                        &route_allocation_blocked_by[t1_entering_route],
                        &movement_graphs[*t2_idx],
                        t2_idx,
                        (*t2_node).clone(),
                    );

                    // or t1 came first.
                    let t1_first_alternatives = advance_until_stops_blocking(
                        &route_allocation_blocked_by[t2_entering_route],
                        &movement_graphs[t1_idx],
                        &t1_idx,
                        t1_node.clone(),
                    );

                    println!(
                        "COnflicts between {}--{:?}--{:?} and {}--{:?}--{:?} ",
                        t1_idx,
                        t1_node,
                        t1_first_alternatives,
                        t2_idx,
                        t2_node,
                        t2_first_alternatives
                    );

                    let t1_init = t1_node == &train_init_routes[t1_idx];
                    let t2_init = t2_node == &train_init_routes[*t2_idx];

                    let t1_first = s.new_bool();
                    let t2_first = s.new_bool();

                    assert!(!(t1_init && t2_init));
                    if t1_init {
                        s.add_clause(&vec![!t2_first]);
                    } else if t2_init {
                        s.add_clause(&vec![!t1_first]);
                    }

                    for t2_out_node in t2_first_alternatives.iter() {
                        let use_precedence = if t2_first_alternatives.len() == 1 {
                            t2_first
                        } else {
                            let v = s.new_bool();
                            s.add_clause(&vec![
                                !t2_first,
                                !train_vars[*t2_idx].node[&t2_out_node].0,
                                v,
                            ]);
                            v
                        };

                        s.add_diff(
                            Some(use_precedence),
                            train_vars[*t2_idx].node[&t2_out_node].1,
                            train_vars[t1_idx].node[t1_node].1,
                            -1,
                        );
                    }

                    for t1_out_node in t1_first_alternatives.iter() {
                        let use_precedence = if t1_first_alternatives.len() == 1 {
                            t1_first
                        } else {
                            let v = s.new_bool();
                            s.add_clause(&vec![
                                !t1_first,
                                !train_vars[t1_idx].node[&t1_out_node].0,
                                v,
                            ]);
                            v
                        };
                        s.add_diff(
                            Some(use_precedence),
                            train_vars[t1_idx].node[&t1_out_node].1,
                            train_vars[*t2_idx].node[t2_node].1,
                            -1,
                        );
                    }

                    s.add_clause(&vec![
                        !train_vars[t1_idx].node[t1_node].0,
                        !train_vars[*t2_idx].node[t2_node].0,
                        t1_first,
                        t2_first,
                    ]);

                    n_conflicts += 1;
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
        s.add_clause(&vec![
            train_vars[train_idx].node[&train_init_routes[train_idx]].0,
        ]);
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

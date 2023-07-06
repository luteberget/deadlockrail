#[allow(unused)]
use log::{debug, info, warn};

use crate::{plan::DeadlockResult, problem::Problem};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque},
    ops::Not,
};
type Occ<'a> = Vec<&'a String>;

pub trait IDLSolver {
    type IntVar: Clone;
    type BoolLit: Not<Output = Self::BoolLit> + Clone + std::fmt::Debug;
    fn solve(&mut self) -> Result<(), ()>;
    fn new_bool(&mut self) -> Self::BoolLit;
    fn new_int(&mut self) -> Self::IntVar;
    fn add_clause(&mut self, xs: &[Self::BoolLit]);
    fn add_diff(&mut self, cond: Option<Self::BoolLit>, x: Self::IntVar, y: Self::IntVar, k: i64);
    fn printname(&self);
}

impl IDLSolver for idl::IdlSolver {
    type BoolLit = idl::Lit;
    type IntVar = idl::DVar;

    fn solve(&mut self) -> Result<(), ()> {
        self.solve().map(|_| ()).map_err(|_| ())
    }

    fn new_bool(&mut self) -> Self::BoolLit {
        self.new_bool()
    }

    fn new_int(&mut self) -> Self::IntVar {
        self.new_int()
    }

    fn add_clause(&mut self, xs: &[Self::BoolLit]) {
        self.add_clause(xs)
    }

    fn add_diff(&mut self, cond: Option<Self::BoolLit>, x: Self::IntVar, y: Self::IntVar, k: i64) {
        self.add_diff(cond, x, y, k);
    }

    fn printname(&self) {
        println!("IDL: rust dpllt with idl solver");
    }
}

impl<'a> IDLSolver for (&'a z3::Context, z3::Solver<'a>) {
    type IntVar = z3::ast::Int<'a>;

    type BoolLit = z3::ast::Bool<'a>;

    fn solve(&mut self) -> Result<(), ()> {
        match self.1.check() {
            z3::SatResult::Unsat => Err(()),
            z3::SatResult::Unknown => panic!("z3 undecided"),
            z3::SatResult::Sat => Ok(()),
        }
    }

    fn new_bool(&mut self) -> Self::BoolLit {
        z3::ast::Bool::fresh_const(&self.0, "y")
    }

    fn new_int(&mut self) -> Self::IntVar {
        z3::ast::Int::fresh_const(&self.0, "x")
    }

    fn add_clause(&mut self, xs: &[Self::BoolLit]) {
        let refs = xs.iter().collect::<Vec<_>>();
        self.1.assert(&z3::ast::Bool::or(self.0, &refs));
    }

    fn add_diff(&mut self, cond: Option<Self::BoolLit>, x: Self::IntVar, y: Self::IntVar, k: i64) {
        let diff = z3::ast::Int::le(&(x - y), &z3::ast::Int::from_i64(self.0, k));

        if let Some(cond) = cond.as_ref() {
            self.1.assert(&z3::ast::Bool::implies(cond, &diff));
        } else {
            self.1.assert(&diff);
        }
    }

    fn printname(&self) {
        println!("IDL: z3");
    }
}

pub fn solve<S: IDLSolver>(problem: &Problem, mut s: S) -> DeadlockResult {
    s.printname();
    let empty_node: Vec<&String> = vec![];
    let p_init = hprof::enter("cycle init");
    let mut movement_graphs = Vec::new();
    let mut route_allocation_blocked_by: HashMap<&String, HashSet<(usize, Vec<&String>)>> =
        Default::default();

    let mut train_init_routes = Vec::new();
    let mut unit_clauses = Vec::new();

    for (train_idx, train) in problem.trains.iter().enumerate() {
        let g = {
            // In Sasso 2023 bencmarks, the original list of initial routes is not consistently ordered.
            let init_routes = sorted_initial_routes(train);
            assert!(!train.initial_routes.is_empty());
            assert!(!init_routes.is_empty());
            train_init_routes.push(init_routes.clone());

            let mut graph: BTreeMap<Occ, BTreeSet<(Occ, Occ)>> = Default::default();
            graph.insert(init_routes.clone(), Default::default());

            let mut states = vec![init_routes];
            while let Some(state) = states.pop() {
                // Make a look-up structure of which routes cannot be allocated when this train is in this node.
                let entering = state.last().unwrap();
                // println!("STATE {:?} entering {:?}", state, entering);
                let mut remaining_length = train.routes[*entering].train_length;
                for route in state.iter().copied().rev() {
                    let r = &train.routes[route];
                    for unconditional_conflict in r
                        .unconditional_conflicts
                        .iter()
                        .chain(std::iter::once(*entering))
                    {
                        // println!(
                        //     "{} blocked by {:?}",
                        //     unconditional_conflict,
                        //     (train_idx, state.clone())
                        // );
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
                        // println!(
                        //     "{} blocked by {:?}",
                        //     allocation_conflict,
                        //     (train_idx, state.clone())
                        // );
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

    struct TrainVars<'a, S: IDLSolver> {
        node: BTreeMap<&'a Occ<'a>, (S::BoolLit, S::IntVar)>,
        // edge: BTreeMap<(&'a Occ<'a>, &'a Occ<'a>), S::BoolLit>,
    }

    let mut train_vars: Vec<TrainVars<S>> = Vec::new();

    // Need to choose a path through the graph
    for train_idx in 0..problem.trains.len() {
        // let init_node = &inits[train_idx];
        let movement_graph = &movement_graphs[train_idx];
        let mut node_vars: BTreeMap<&Occ, (S::BoolLit, S::IntVar)> = Default::default();

        // let mut train_edges: BTreeMap<(&Occ, &Occ), S::BoolLit> = Default::default();
        node_vars.insert(&train_init_routes[train_idx], (s.new_bool(), s.new_int()));
        let mut unvisited = vec![&train_init_routes[train_idx]];

        unit_clauses.push(vec![node_vars[&train_init_routes[train_idx]].0.clone()]);

        // DFS through the movement graph and add variables and flow constraints.
        while let Some(state) = unvisited.pop() {
            let is_black_hole = movement_graph[state].is_empty();
            assert!(state.is_empty() == is_black_hole);

            if is_black_hole {
                // No constraints on the black hole node.

                continue;
            }

            let (source_var, source_time) = node_vars[&state].clone();
            let mut alternatives = vec![!source_var.clone()];
            for (next_node, _) in movement_graph[state].iter() {
                let (target_var, target_time) = node_vars
                    .entry(next_node)
                    .or_insert_with(|| {
                        unvisited.push(next_node);
                        (s.new_bool(), s.new_int())
                    })
                    .clone();

                let edge_active = s.new_bool();
                s.add_clause(&[
                    !(source_var.clone()),
                    !(target_var.clone()),
                    edge_active.clone(),
                ]);

                alternatives.push(target_var.clone());

                s.add_diff(Some(edge_active), source_time.clone(), target_time, -1);
            }

            assert!(alternatives.len() >= 2);
            s.add_clause(&alternatives);
        }

        // Each node in the movement graph should now have an associated bool variable.
        assert!(movement_graph.len() == node_vars.len());
        train_vars.push(TrainVars {
            node: node_vars,
            // edge: train_edges,
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

    type C<'a> = (usize, &'a Vec<&'a String>, BTreeSet<Vec<&'a String>>);
    let mut added_conflicts: BTreeSet<(C, C)> = Default::default();

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
                    // println!(
                    //     "train {}  t2_node {:?} t2_entering {:?}",
                    //     t2_idx, t2_node, t2_entering_route
                    // );

                    let t1_init = t1_node == &train_init_routes[t1_idx];
                    let t2_init = t2_node == &train_init_routes[*t2_idx];

                    if t1_init && t2_init {
                        warn!("conflicting initial routes between  train {} ({:?} and train {} ({:?})",
                        t1_idx, t1_node, t2_idx, t2_node);
                        info!("DEADLOCKED");

                        return DeadlockResult::Deadlocked(());
                        // continue;
                    };

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

                    // println!(
                    //     "COnflicts between {}--{:?}--{:?} and {}--{:?}--{:?} ",
                    //     t1_idx,
                    //     t1_node,
                    //     t1_first_alternatives,
                    //     t2_idx,
                    //     t2_node,
                    //     t2_first_alternatives
                    // );

                    let c1: C = (
                        t1_idx,
                        t1_node,
                        t2_first_alternatives.iter().cloned().collect(),
                    );
                    let c2: C = (
                        *t2_idx,
                        t2_node,
                        t1_first_alternatives.iter().cloned().collect(),
                    );
                    let (c1, c2) = if c1.0 < c2.0 { (c1, c2) } else { (c2, c1) };

                    let new_conflict = added_conflicts.insert((c1, c2));
                    if !new_conflict {
                        // println!(
                        //     "REDUNDANT COnflicts between {}--{:?}--{:?} and {}--{:?}--{:?} ",
                        //     t1_idx,
                        //     t1_node,
                        //     t1_first_alternatives,
                        //     t2_idx,
                        //     t2_node,
                        //     t2_first_alternatives
                        // );
                        continue;
                    }

                    let t1_first = s.new_bool();
                    let t2_first = s.new_bool();

                    assert!(!(t1_init && t2_init));
                    if t1_init {
                        unit_clauses.push(vec![!(t2_first.clone())]);
                    } else if t2_init {
                        unit_clauses.push(vec![!(t1_first.clone())]);
                    }

                    for t2_out_node in t2_first_alternatives.iter() {
                        let use_precedence = if t2_first_alternatives.len() == 1 {
                            t2_first.clone()
                        } else {
                            let v = s.new_bool();
                            s.add_clause(&vec![
                                !t2_first.clone(),
                                !train_vars[*t2_idx].node[&t2_out_node].0.clone(),
                                v.clone(),
                            ]);
                            v
                        };

                        s.add_diff(
                            Some(use_precedence),
                            train_vars[*t2_idx].node[&t2_out_node].1.clone(),
                            train_vars[t1_idx].node[t1_node].1.clone(),
                            -1,
                        );
                    }

                    for t1_out_node in t1_first_alternatives.iter() {
                        let use_precedence = if t1_first_alternatives.len() == 1 {
                            t1_first.clone()
                        } else {
                            let v = s.new_bool();
                            s.add_clause(&vec![
                                !t1_first.clone(),
                                !train_vars[t1_idx].node[&t1_out_node].0.clone(),
                                v.clone(),
                            ]);
                            v
                        };
                        s.add_diff(
                            Some(use_precedence),
                            train_vars[t1_idx].node[&t1_out_node].1.clone(),
                            train_vars[*t2_idx].node[t2_node].1.clone(),
                            -1,
                        );
                    }

                    s.add_clause(&vec![
                        !train_vars[t1_idx].node[t1_node].0.clone(),
                        !train_vars[*t2_idx].node[t2_node].0.clone(),
                        t1_first.clone(),
                        t2_first.clone(),
                    ]);

                    n_conflicts += 1;
                }
            }
        }
    }
    drop(added_conflicts);

    // 2023-07-03 There is a bug in the IDL solver, which doesn't handle
    // any literals being set before adding the difference constraints,
    // which happens when unit clauses are added, interleaved with the
    // conditional difference constraints.
    // Here, we have postponed adding the unit clauses until now, pending
    // a bug fix in the IDL library.

    for c in unit_clauses {
        s.add_clause(&c);
    }

    warn!("n_conflicts={}", n_conflicts);

    drop(p_init);
    let _p = hprof::enter("cycle solve");

    match s.solve() {
        Ok(_model) => {
            info!("LIVE");
            for (train_idx, trainvar) in train_vars.iter().enumerate() {
                // for (node, (on, t)) in trainvar.node.iter() {
                //     debug!(
                //         "train {} node {:?} on={:?}=={} t={:?}=={}",
                //         train_idx,
                //         node,
                //         on,
                //         s.get_bool_value(*on),
                //         t,
                //         s.get_int_value(*t)
                //     );
                // }
                // for ((n1, n2), on) in trainvar.edge.iter() {
                //     debug!(
                //         "train {} EDGE {:?}--->{:?} on={:?}=={}",
                //         train_idx,
                //         n1,
                //         n2,
                //         on,
                //         s.get_bool_value(*on)
                //     );
                // }
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

    // Sometimes there is an initial route that is not described as a route.
    let initial_routes = train
        .initial_routes
        .iter()
        .filter(|r| {
            if !train.routes.contains_key(*r) {
                warn!("");
                return false;
            }

            true
        })
        .collect::<Vec<_>>();

    // println!(
    //     "initial routes for train {}: {:?}",
    //     train.name, train.initial_routes
    // );
    // for i in initial_routes.iter() {
    //     println!("  * {:?}", train.routes[*i]);
    // }

    assert!(initial_routes
        .iter()
        .all(|n| train.routes[*n].next_routes.is_some()));

    // Check that exactly one of the initial routes does not point to one of the other
    // initial routes.
    assert!(
        initial_routes
            .iter()
            .filter(|n| !train.routes[**n]
                .next_routes
                .as_ref()
                .unwrap()
                .iter()
                .any(|r| initial_routes.contains(&r)))
            .count()
            == 1
    );

    // Insert in sorted order
    let mut rs = Vec::new();
    // Insert the last one first
    rs.push(
        *initial_routes
            .iter()
            .find(|n| {
                !train.routes[**n]
                    .next_routes
                    .as_ref()
                    .unwrap()
                    .iter()
                    .any(|r| initial_routes.contains(&r))
            })
            .unwrap(),
    );

    // Insert the one poining to the previous.
    while rs.len() < initial_routes.len() {
        rs.push(
            *initial_routes
                .iter()
                .find(|n| {
                    train.routes[**n]
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

use crate::plan::{RouteId, TrainId};
use crate::{plan::Plan, problem::*};
use log::*;
use satcoder::prelude::*;
use satcoder::symbolic::FinSet;
use satcoder::{Bool, Lit};
use std::collections::HashSet;
use std::{collections::HashMap, iter::once};
use velcro::{iter, vec};

#[derive(Clone, Copy)]
pub struct StateConstraintSettings {
    pub global_progress_constraint: bool,
    pub local_early_progress_constraint: bool,
}

pub struct State<L: satcoder::Lit> {
    routes: HashMap<RouteId, FinSet<L, Option<TrainId>>>,
    trains_finished: HashMap<TrainId, satcoder::Bool<L>>,
}

pub fn has_progressed_by_length<L: Lit>(
    s: &mut impl SatInstance<L>,
    problem: &Problem,
    train: TrainId,
    route: &str,
    occ: &HashMap<RouteId, FinSet<L, Option<TrainId>>>,
    length: u64,
) -> Vec<Bool<L>> {
    if let Some(nexts) = problem.trains[train].routes[route].next_routes.as_ref() {
        let mut alternatives = vec![];
        if nexts.is_empty() {
            error!("Train {} route {} has empty nexts {:?}", train, route, problem.trains[train].routes[route]);
        }
        assert!(!nexts.is_empty());
        for next in nexts.iter() {
            trace!("train {} next {}", train, next);
            let next_route = &problem.trains[train].routes[next];
            let occupied = occ[next].has_value(&Some(train));
            if next_route.route_length >= length {
                alternatives.push(occupied);
            } else {
                let freeable_paths = has_progressed_by_length(
                    s,
                    problem,
                    train,
                    next,
                    occ,
                    length - next_route.route_length,
                );
                let any_next = s.or_literal(freeable_paths);
                alternatives.push(s.and_literal(iter![occupied, any_next]));
            }
        }
        alternatives
    } else {
        vec![true.into()]
    }
}

/// Calculates the maximum number of planning steps that can be used in the
/// problem when each step must move at least one train by at least one route.
/// A planning problem with this number of steps must then be guaranteed to
/// contain all possible solutions.  Assumes that the infrastructure is acyclic.
pub fn ub_steps(problem: &Problem) -> usize {
    let steps = problem
        .trains
        .iter()
        .map(|t| {
            let mut max_dist = 0usize;
            let mut dist = 0usize;
            let (mut set, mut next_set) = (
                t.initial_routes.iter().collect::<HashSet<_>>(),
                HashSet::new(),
            );
            while !set.is_empty() {
                for r in set.drain() {
                    match t.routes.get(r).and_then(|r| r.next_routes.as_ref()) {
                        Some(nexts) => next_set.extend(nexts.iter()),
                        None => max_dist = max_dist.max(dist),
                    }
                }
                std::mem::swap(&mut set, &mut next_set);
                dist += 1;
            }

            max_dist
        })
        .sum();

    info!("Problem UB steps: {}", steps);
    steps
}

pub fn mk_state<L: Lit>(
    s: &mut impl SatInstance<L>,
    prev: &State<L>,
    problem: &Problem,
    n: usize,
    settings: StateConstraintSettings,
) -> State<L> {
    let mut routes: HashMap<RouteId, Vec<TrainId>> = HashMap::new();
    for (train_id, train) in problem.trains.iter().enumerate() {
        for (route_id, _route) in train.routes.iter() {
            routes
                .entry(route_id.clone())
                .or_insert_with(Vec::new)
                .push(train_id);
        }
    }
    let routes = routes.into_iter().collect::<Vec<_>>();
    trace!("Routes {:?}", routes);
    // Each route selects a train (or none)
    let occ = routes
        .iter()
        .map(|(r, t)| {
            (
                r.clone(),
                FinSet::new(
                    s,
                    std::iter::once(None)
                        .chain(t.iter().copied().map(Some))
                        .collect(),
                ),
            )
        })
        .collect::<HashMap<RouteId, FinSet<L, Option<TrainId>>>>();

    // debug!("Route occupation variables:");
    // for occ in occ.iter() {
    //     debug!(
    //         "route {} has {} alternatives",
    //         occ.0,
    //         occ.1.domain().count()
    //     );
    // }

    // ROUTE EXCLUSIONS

    // Three stages of exclusions:
    //  1. pair of routes can never be active at the same time
    //  2. pair of routes must be used by at most one train
    //  3. pair of routes exclusive based on trains
    // for (r1idx, (r1, ts1)) in routes.iter().enumerate() {
    //     for (r2, ts2) in routes[(r1idx + 1)..].iter() {

    //         // 3: train-based

    //         for t1 in ts1.iter() {
    //             for incompat in problem.trains[*t1].routes[r1].incompatible_routes.iter() {
    //                 if incompat == r2 {
    //                     s.add_clause(vec![!occ[r1].has_value(&Some(*t1)), occ[r2].has_value(&None), occ[r2].has_value(&Some(*t1))]);
    //                 }
    //             }
    //         }

    //     }
    // }

    // NAIVE ROUTE EXCLUSIONS (no shortcuts implemented yet)
    let mut num_naive_exclusions = 0;
    for (t1, train) in problem.trains.iter().enumerate() {
        for (r1, routedata) in train.routes.iter() {
            for r2 in routedata.unconditional_conflicts.iter() {
                // t1r1 => t1r2 or r2_empty

                if !occ.contains_key(r2) {
                    // warn!("Route {} is unreachable (r1={} t1={})", r2, r1, t1);
                    continue;
                }

                assert!(r1 != r2);
                if r1 == r2 {
                    continue; // same-route exclusion is implicit in the route-occupation formulation
                }

                // trace!(
                //     "constraint for train {} in route {} excludes route {}",
                //     t1,
                //     r1,
                //     r2
                // );

                s.add_clause(vec![
                    !occ[r1].has_value(&Some(t1)),
                    occ[r2].has_value(&Some(t1)),
                    occ[r2].has_value(&None),
                ]);

                num_naive_exclusions += 1;
            }

            for r2 in routedata.allocation_conflicts.iter() {
                // r2 cannot be allocated if t1 is in r1

                if !occ.contains_key(r2) {
                    // warn!("Route {} is unreachable (r1={} t1={})", r2, r1, t1);
                    continue;
                }

                for t2 in occ[r2].domain().filter_map(|x| *x) {
                    if t1 == t2 {
                        continue; // no constraint between the same train
                    }

                    println!(
                        "Cannot allocate r{} to t{} while t{} is in r{}",
                        r2, t2, t1, r1
                    );

                    // let t1_continued = problem.trains[t1].routes[r1]
                    //     .next_routes
                    //     .iter()
                    //     .flat_map(|nxs| nxs.iter().map(|n| occ[n].has_value(&Some(t1))));

                    let length = problem.trains[t1].routes[r1].train_length
                        - problem.trains[t1].routes[r1].route_length_without_switch;

                    let t1_continued = has_progressed_by_length(s, problem, t1, r1, &occ, length);

                    s.add_clause(iter![
                        !occ[r1].has_value(&Some(t1)),
                        ..t1_continued,
                        prev.routes[r2].has_value(&Some(t2)),
                        !occ[r2].has_value(&Some(t2)),
                    ]);
                    num_naive_exclusions += 1;
                }
            }
        }
    }
    debug!("naive exclusions {}", num_naive_exclusions);

    // TRAIN CONSISTENCY

    // 1: train cannot take more than one route
    for (route, route_value) in occ.iter() {
        for train in route_value.domain().filter_map(|x| *x) {
            if let Some(nexts) = problem.trains[train].routes[route].next_routes.as_ref() {
                s.assert_at_most_one(nexts.iter().map(|r| occ[r].has_value(&Some(train))));
            }
        }
    }

    // // 1b: cannot take more than one route even across states
    // //  (not relevant when always freeing when train has passed).
    // if !settings.local_early_progress_constraint {
    //     for (route, route_value) in occ.iter() {
    //         for train in route_value.domain().filter_map(|x| *x) {
    //             if let Some(nexts) = problem.trains[train].routes[route].next_routes.as_ref() {
    //                 for i in 0..nexts.len() {
    //                     for j in (0..nexts.len()).filter(|j| *j != i) {
    //                         s.add_clause(iter![
    //                             !occ[&nexts[i]].has_value(&Some(train)),
    //                             !prev.routes[&nexts[j]].has_value(&Some(train))
    //                         ]);
    //                     }
    //                 }
    //             }
    //         }
    //     }
    // }

    // 2: train cannot appear in a route without already being in the previous route
    let mut all_allocations = Vec::new();
    for (route, route_value) in occ.iter() {
        for train in route_value.domain().filter_map(|x| *x) {
            // possible train sources
            let alternatives = problem.trains[train]
                .routes
                .iter()
                .filter_map(|(r, data)| data.next_routes.as_ref().map(|nexts| (r, nexts)))
                .filter_map(|(r, nexts)| nexts.iter().any(|r| r == route).then(|| r))
                .collect::<Vec<&String>>();

            trace!("Alternatives for t{} r{}: {:?}", train, route, alternatives);

            let alt_lits = alternatives
                .into_iter()
                .map(|route| occ[route].has_value(&Some(train)))
                .collect::<Vec<_>>();

            let allocated = s.and_literal(iter![
                occ[route].has_value(&Some(train)),
                !prev.routes[route].has_value(&Some(train))
            ]);

            all_allocations.push(allocated);
            s.add_clause(iter![
                !allocated,
                ..alt_lits.iter().copied()
            ]);

            // // When de-allocating, the prevs must also be deallocated.
            // if !settings.local_early_progress_constraint {
            //     let deallocated = s.and_literal(iter![
            //         !occ[route].has_value(&Some(train)),
            //         prev.routes[route].has_value(&Some(train))
            //     ]);

            //     for alt in alt_lits.iter().copied() {
            //         s.add_clause(iter![!deallocated, !alt]);
            //     }
            // }
        }
    }

    // 3: cannot disappear without having continued to at least one of next_routes,
    //    and they together must contain the train's length

    for (route, route_value) in occ.iter() {
        for train in route_value.domain().filter_map(|x| *x) {
            // println!("OCC {:?}", occ);

            let train_length = problem.trains[train].routes[route].train_length;
            // println!("Creating freeable for {} {} l={}", route, train, train_length);
            let freeable_paths =
                has_progressed_by_length(s, problem, train, route, &prev.routes, train_length);
            // println!(" got {} alternatives {:?}", freeable_paths.len(), freeable_paths);
            let free = s.or_literal(freeable_paths);

            let was_occupied = prev.routes[route].has_value(&Some(train));
            let is_occupied = occ[route].has_value(&Some(train));
            s.add_clause(iter![
                !was_occupied, // IF the train was there
                is_occupied,   // It is still there
                free           // OR we could free it
            ]);

            s.add_clause(iter![
                !was_occupied, // IF the train was there
                !is_occupied,  // It has now left
                !free          // OR we COULDN'T free it
            ]);
        }
    }

    // PROGRESS CONSTRAINS

    // Global progress condition
    // NAIVE: at least one allocation
    if settings.global_progress_constraint {
        s.add_clause(all_allocations);
    }
    // TODO improve this    ?

    // Forced progress:
    // when a next-route is allocated in step n (n>1)
    // a conflicting route must have been allocated in the step before (by another train)
    // println!("Forced progress");
    if settings.local_early_progress_constraint {
        forced_progress(n, &occ, problem, s, prev);
    }
    // println!("Forced progress done");

    // Check if trains have finished.
    let mut trains_finished = HashMap::new();
    for (train_id, train) in problem.trains.iter().enumerate() {
        let alternatives = train
            .routes
            .iter()
            .filter_map(|(id, r)| r.next_routes.is_none().then(|| id))
            .filter_map(|r| occ.get(r))
            .map(|set| set.has_value(&Some(train_id)));

        let finished = s.new_var();
        s.add_clause(
            once(!finished)
                .chain(once(prev.trains_finished[&train_id]))
                .chain(alternatives),
        );
        trains_finished.insert(train_id, finished);
    }

    State {
        routes: occ,
        trains_finished,
    }
}

fn forced_progress<L: Lit>(
    n: usize,
    occ: &HashMap<String, FinSet<L, Option<usize>>>,
    problem: &Problem,
    s: &mut impl SatInstance<L>,
    prev: &State<L>,
) {
    if n >= 2 {
        for (route, route_value) in occ.iter() {
            for train in route_value.domain().filter_map(|x| *x) {
                // possible train sources
                let alternatives = problem.trains[train]
                    .routes
                    .iter()
                    .filter_map(|(r, data)| data.next_routes.as_ref().map(|nexts| (r, nexts)))
                    .filter_map(|(r, nexts)| nexts.iter().any(|r| r == route).then(|| r))
                    .collect::<Vec<&String>>();
                trace!("Alternatives for t{} r{}: {:?}", train, route, alternatives);

                let had_prev_route = alternatives
                    .into_iter()
                    .map(|r| prev.routes[r].has_value(&Some(train)));

                let had_prev_route = s.or_literal(had_prev_route);

                let allocated = s.and_literal(
                    once(had_prev_route).chain(
                        once(occ[route].has_value(&Some(train)))
                            .chain(once(!prev.routes[route].has_value(&Some(train)))),
                    ),
                );

                let conflicting_routes = iter![
                    route,
                    ..problem.trains[train].routes[route]
                        .unconditional_conflicts
                        .iter(),
                    ..problem.trains[train].routes[route]
                        .allocation_conflicts
                        .iter(),
                ];
                let had_conflicting_alloc = conflicting_routes
                    .filter(|r| prev.routes.contains_key(*r))
                    .map(|r| {
                        s.and_literal(
                            once(!prev.routes[r].has_value(&Some(train)))
                                .chain(once(!prev.routes[r].has_value(&None))),
                        )
                    })
                    .collect::<Vec<_>>();

                s.add_clause(iter![!allocated, ..had_conflicting_alloc,]);
            }
        }
    }
}

pub fn initial_state<L: satcoder::Lit>(s: &mut impl SatInstance<L>, problem: &Problem) -> State<L> {
    let mut occ: HashMap<RouteId, Option<TrainId>> = HashMap::new();

    // Put the initial routes as known values
    for (train_id, train) in problem.trains.iter().enumerate() {
        for route_id in train.initial_routes.iter() {
            assert!(!occ.contains_key(route_id));
            occ.insert(route_id.clone(), Some(train_id));
        }
    }

    // Put all other routes in as not occupied
    for (_train_id, train) in problem.trains.iter().enumerate() {
        for (route_id, _route) in train.routes.iter() {
            if !occ.contains_key(route_id) {
                occ.insert(route_id.clone(), None);
            }
        }
    }

    let occ = occ
        .into_iter()
        .map(|(r, t)| (r, FinSet::new(s, vec![t])))
        .collect();

    State {
        routes: occ,
        trains_finished: problem
            .trains
            .iter()
            .map(|t| {
                t.initial_routes
                    .iter()
                    .any(|r| {
                        t.routes
                            .get(r)
                            .map(|r| r.next_routes.is_none())
                            .unwrap_or(false)
                    })
                    .into()
            })
            .enumerate()
            .collect(),
    }
}

pub fn goal_condition<L: satcoder::Lit>(state: &State<L>) -> Option<Vec<satcoder::Bool<L>>> {
    let mut condition = Vec::new();
    for (_id, finished) in state.trains_finished.iter() {
        match finished {
            Bool::Const(true) => {}
            Bool::Const(false) => {
                /* cannot be goal */
                return None;
            }
            l => {
                condition.push(*l);
            }
        }
    }
    Some(condition)
}

pub fn model_to_plan<L: satcoder::Lit>(
    states: &[State<L>],
    problem: &Problem,
    model: &dyn SatModel<Lit = L>,
) -> Plan {
    let mut steps = Vec::new();
    for step in states.iter() {
        let mut routes = HashMap::new();
        for train in problem.trains.iter() {
            for (route_name, _route) in train.routes.iter() {
                if let Some(value) = step.routes.get(route_name) {
                    routes.insert(route_name.clone(), *model.value(value));
                }
            }
        }
        let mut routes = routes.into_iter().collect::<Vec<_>>();
        routes.sort();
        steps.push(routes);
    }

    Plan { steps }
}

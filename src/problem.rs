use crate::raw2021_problem;
use crate::raw2023_problem;
use crate::raw2023_problem::RouteLengthExclusion;
use log::*;
use std::collections::HashMap;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Problem {
    pub trains: Vec<Train>,
}

pub type RouteRef = String;
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Train {
    pub name: String,
    pub initial_routes: Vec<RouteRef>,
    pub routes: HashMap<RouteRef, TrainRoute>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct TrainRoute {
    pub train_length: u64,
    pub route_length: u64,
    pub route_length_without_switch: u64,
    pub unconditional_conflicts: Vec<RouteRef>,
    pub allocation_conflicts: Vec<RouteRef>,
    pub next_routes: Option<Vec<RouteRef>>,
}

pub fn convert_raw2023(problem: &raw2023_problem::Problem) -> Problem {
    // Let's check a few assumptions first.

    warn!("multi_initial Train {}", problem.trains.len());
    // Are there more than two conflict sets (different
    // releases at different lengths) ?

    // Initial configuration with multiple trains in routes
    let mut multi_initial: HashMap<String, Vec<&raw2023_problem::Train>> = Default::default();
    for train in problem.trains.iter() {
        for initial in train.initial_routes.iter().cloned() {
            multi_initial.entry(initial).or_default().push(train);
        }
    }
    multi_initial.retain(|_k, v| v.len() > 1);
    warn!(
        "multi_initial map {:?}",
        multi_initial
            .values()
            .map(|v| v.iter().map(|t| t.id.clone()).collect::<Vec<_>>())
            .collect::<Vec<_>>()
    );
    warn!("multi_initial map {:?}", multi_initial);

    let multi_initial_trains = multi_initial
        .into_values()
        .flat_map(move |t| t.into_iter().map(move |t| t.id.clone()))
        .collect::<std::collections::HashSet<_>>();


    let mut trains: Vec<Train> = Vec::new();

    // let mut additional_conflicts: HashMap<String, Vec<String>> = Default::default();

    for train in problem.trains.iter() {
        // The 2023 benchmark set does not have dummy trains.
        assert!(!train.is_dummy);

        assert!(train.final_routes.is_empty());
        assert!(
            train.initial_multitrain_prev_trainid.is_empty()
                || train.initial_multitrain_prev_trainid.len() == 1
        );

        let mut routes: HashMap<String, TrainRoute> = HashMap::new();

        let use_multi_initial = multi_initial_trains.contains(&train.id)
            .then(|| format!("init_{}", train.id));

        let initial_routes = if let Some(init) = use_multi_initial.as_ref() {
            warn!("Train {} uses multi_initial", train.id);
            routes.insert(
                init.clone(),
                TrainRoute {
                    route_length: 0,
                    route_length_without_switch: 0,
                    train_length: 0,
                    unconditional_conflicts: vec![],
                    allocation_conflicts: vec![],
                    next_routes: Some(vec![]),
                },
            );
            vec![init.clone()]
        } else {
            train.initial_routes.clone()
        };

        let trainroutes = problem
            .train_routes
            .iter()
            .filter(|r| r.train == train.id)
            .collect::<Vec<_>>();
        for trainroute in trainroutes {
            // A quirk in the input data is that there are two length specifications for each route correspondence.
            // The first length is lower and lists unconditional conflicts.
            // The second length is higher and lists conflicts that only apply when the train length exceeds the length in the exclusion row.

            let mut conflict_sets = problem
                .conflicts
                .iter()
                .filter(|c| c.route == trainroute.route)
                .collect::<Vec<_>>();

            let is_dummy_route = trainroute.route.starts_with("100");
            assert!(conflict_sets.is_empty() == is_dummy_route);
            assert!(trainroute.is_black_hole == is_dummy_route);

            if !trainroute.is_black_hole && trainroute.next_routes.is_empty() {
                warn!("Train route has no following routes (and is not a black hole)");
            }

            let extra_conflict_set;
            if conflict_sets.len() == 1 {
                warn!(
                    "Only one conflict set t{}-r{}: {:?}",
                    trainroute.train, trainroute.route, conflict_sets
                );

                extra_conflict_set = Some(RouteLengthExclusion {
                    conflicts: vec![],
                    length: conflict_sets[0].length + 1,
                    route: conflict_sets[0].route.clone(),
                });
                conflict_sets.push(extra_conflict_set.as_ref().unwrap());
            }

            let route = problem
                .routes
                .iter()
                .find(|r| r.id == trainroute.route)
                .unwrap();

            assert!(!route.is_unusable);
            if is_dummy_route {
                assert!(trainroute.is_black_hole);
                routes.insert(
                    trainroute.route.clone(),
                    TrainRoute {
                        route_length: trainroute.length,
                        train_length: trainroute.length,
                        route_length_without_switch: trainroute.length,
                        unconditional_conflicts: vec![],
                        allocation_conflicts: vec![],
                        next_routes: None,
                    },
                );
            } else {
                assert!(conflict_sets[0].conflicts.contains(&trainroute.route));

                if conflict_sets.len() != 2 {
                    warn!(
                        "Cnflicts sets for route {}: {:?}",
                        trainroute.route, conflict_sets
                    );
                }

                // if conflict_sets.is_empty() {
                //     warn!("No conflicts found for route {}", trainroute.route);
                // } else if conflict_sets.len() != 2 {
                //     panic!("Conflicts should always be listed in two length levels.");
                // }

                assert!(conflict_sets[0].length < conflict_sets[1].length);

                let mut unconditional_conflicts = conflict_sets[0].conflicts.clone();
                unconditional_conflicts.retain(|c| c != &trainroute.route); // Self-conflicts are implicit

                let train_is_long = trainroute.length > conflict_sets[0].length;

                let allocation_conflicts = if train_is_long {
                    let mut v = conflict_sets[1].conflicts.clone();
                    v.retain(|c| c != &trainroute.route); // Self-conflicts are implicit
                    v
                } else {
                    vec![]
                };

                let route_length_without_switch = conflict_sets[0].length;

                // Is this an initial route, shared by multiple trains?
                if use_multi_initial.is_some() && train.initial_routes.contains(&trainroute.route) {
                    let init = use_multi_initial.as_ref().unwrap();
                    assert!(!is_dummy_route);

                    println!("MULTI {} {}", trainroute.train, trainroute.route);

                    let init_route = routes.get_mut(init).unwrap();
                    init_route.train_length = init_route.train_length.max(trainroute.length);
                    init_route
                        .unconditional_conflicts
                        .extend(conflict_sets[0].conflicts.iter().cloned());

                    // Is this the most advanced initial route?
                    if !trainroute
                        .next_routes
                        .iter()
                        .any(|r| train.initial_routes.contains(r))
                    {
                        // Is there a train ahead?
                        let mut curr_train_id = &train.id;

                        while let Some(train_ahead) = problem
                            .trains
                            .iter()
                            .find(|t| &t.id == curr_train_id)
                            .unwrap()
                            .initial_multitrain_prev_trainid
                            .iter()
                            .nth(0)
                        {
                            // Add the other train's init route to pass through.
                            routes.insert(
                                format!("init_{}", train_ahead),
                                TrainRoute {
                                    train_length: 0,
                                    route_length: 0,
                                    route_length_without_switch: 0,
                                    unconditional_conflicts: vec![],
                                    allocation_conflicts: vec![],
                                    next_routes: Some(vec![]),
                                },
                            );

                            // Set the previous route's nextroute to this.
                            let r = routes.get_mut(&format!("init_{}", curr_train_id)).unwrap();
                            r.next_routes = Some(vec![format!("init_{}", train_ahead)]);

                            curr_train_id = train_ahead;
                        }

                        // Now, curr_train_id is the train in front, and we should
                        // enter the original next routes.

                        let r = routes.get_mut(&format!("init_{}", curr_train_id)).unwrap();
                        r.next_routes = Some(trainroute.next_routes.iter().cloned().collect());
                    }
                }

                assert!(conflict_sets[0].conflicts.contains(&trainroute.route));
                routes.insert(
                    trainroute.route.clone(),
                    TrainRoute {
                        route_length: conflict_sets[1].length,
                        train_length: trainroute.length,
                        route_length_without_switch,
                        unconditional_conflicts,
                        allocation_conflicts,
                        next_routes: (!trainroute.next_routes.is_empty())
                            .then(|| trainroute.next_routes.clone()),
                    },
                );
            }
        }

        if routes.is_empty() {
            warn!("No routes found for train {}/{}", train.id, train.name);
        }

        // Check that we made the initial route correctly
        if let Some(init) = use_multi_initial.as_ref() {
            assert!(routes[init].train_length > 0);
            assert!(routes[init].allocation_conflicts.is_empty());
            assert!(!routes[init].unconditional_conflicts.is_empty());
            assert!(!routes[init].next_routes.as_ref().unwrap().is_empty());
        }

        trains.push(Train {
            name: train.id.clone(),
            initial_routes,
            routes,
        });
    }

    trains.sort_by_key(|k| k.name.clone());
    let mut summary = String::new();
    for train in trains.iter() {
        summary.push_str(&format!(
            "t{}   @{}",
            train.name,
            train.initial_routes.join(", ")
        ));
        let mut routes = train.routes.iter().map(|(k, v)| (k, v)).collect::<Vec<_>>();
        routes.sort_by_key(|(k, _)| (*k).clone());
        for (id, route) in routes.iter() {
            summary.push('\n');
            summary.push_str(&format!(
                "  r{}   x{}   y{}  -> {}",
                id,
                route.unconditional_conflicts.join(",x"),
                route.allocation_conflicts.join(",y"),
                route
                    .next_routes
                    .as_ref()
                    .map(|r| r.join(", "))
                    .unwrap_or_else(|| "--->".to_string())
            ));
        }
        summary.push('\n');
    }

    debug!("Summary:\n{}", summary);

    // todo!("assumptions ok");
    Problem { trains }
}

pub fn convert_raw2021(problem: &raw2021_problem::Problem) -> Problem {
    for route in problem.routes.iter() {
        assert!(!route.is_multi_train, "Multi-train routes not supported");
        assert!(!route.is_siding, "Siding property not supported");
        assert!(!route.is_unusable, "Unusable property not supported");
        // TODO Any use for station_or_track_id?
        // TODO Any use for is_final_point_in_station ?
    }

    let mut trains: Vec<Train> = Vec::new();
    for train in problem.trains.iter() {
        if train.is_dummy {
            trace!("Skipping dummy train {}/{}", train.id, train.name);
            continue;
        }
        let mut routes: HashMap<String, TrainRoute> = HashMap::new();

        let trainroutes = problem
            .train_routes
            .iter()
            .filter(|r| r.train == train.id)
            .collect::<Vec<_>>();
        for trainroute in trainroutes {
            // A quirk in the input data is that there are two length specifications for each route correspondence.
            // The first length is lower and lists unconditional conflicts.
            // The second length is higher and lists conflicts that only apply when the train length exceeds the length in the exclusion row.

            let conflict_sets = problem
                .conflicts
                .iter()
                .filter(|c| c.route == trainroute.route)
                .collect::<Vec<_>>();
            if conflict_sets.is_empty() {
                warn!("No conflicts found for route {}", trainroute.route);
            } else if conflict_sets.len() != 2 {
                panic!("Conflicts should always be listed in two length levels.");
            }
            assert!(trainroute.is_black_hole == trainroute.next_routes.is_empty());
            assert!(conflict_sets[0].length < conflict_sets[1].length);

            let mut unconditional_conflicts = conflict_sets[0].conflicts.clone();
            unconditional_conflicts.retain(|c| c != &trainroute.route); // Self-conflicts are implicit

            let train_is_long = trainroute.length > conflict_sets[0].length;

            let allocation_conflicts = if train_is_long {
                let mut v = conflict_sets[1].conflicts.clone();
                v.retain(|c| c != &trainroute.route); // Self-conflicts are implicit
                v
            } else {
                vec![]
            };

            let route_length_without_switch = conflict_sets[0].length;
            // assert!(switch_length == 1);

            routes.insert(
                trainroute.route.clone(),
                TrainRoute {
                    route_length: conflict_sets[1].length,
                    train_length: trainroute.length,
                    route_length_without_switch,
                    unconditional_conflicts,
                    allocation_conflicts,
                    next_routes: (!trainroute.is_black_hole)
                        .then(|| trainroute.next_routes.clone()),
                },
            );
        }

        if routes.is_empty() {
            warn!("No routes found for train {}/{}", train.id, train.name);
        }

        trains.push(Train {
            name: train.id.clone(),
            initial_routes: train.initial_routes.clone(),
            routes,
        });
    }

    trains.sort_by_key(|k| k.name.clone());
    let mut summary = String::new();
    for train in trains.iter() {
        summary.push_str(&format!(
            "t{}   @{}",
            train.name,
            train.initial_routes.join(", ")
        ));
        let mut routes = train.routes.iter().map(|(k, v)| (k, v)).collect::<Vec<_>>();
        routes.sort_by_key(|(k, _)| (*k).clone());
        for (id, route) in routes.iter() {
            summary.push('\n');
            summary.push_str(&format!(
                "  r{}   x{}   y{}  -> {}",
                id,
                route.unconditional_conflicts.join(",x"),
                route.allocation_conflicts.join(",y"),
                route
                    .next_routes
                    .as_ref()
                    .map(|r| r.join(", "))
                    .unwrap_or_else(|| "--->".to_string())
            ));
        }
        summary.push('\n');
    }

    debug!("Summary:\n{}", summary);

    Problem { trains }
}

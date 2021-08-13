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

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[derive(Clone)]
pub struct TrainRoute {
    pub train_length: u64,
    pub route_length: u64,
    pub switch_length: u64,
    pub unconditional_conflicts: Vec<RouteRef>,
    pub allocation_conflicts: Vec<RouteRef>,
    pub next_routes: Option<Vec<RouteRef>>,
}

#[cfg(test)]
mod tests {
    

    use super::*;

    #[test]
    pub fn two_track() {
        let l_line_route = 100;
        for l_station_route in [5] {
            for n_line_routes in (10..=2000).step_by(10) {
                for l_train in [10] {
                    let is_sat = l_train <= l_station_route;
                    let problem = generate_two_track_instance(l_line_route, l_station_route, n_line_routes, l_train);

                    assert!(!is_sat);

                    println!(
                        "l_station {} n_line_routes {} l_train {} is_sat {}",
                        l_station_route, n_line_routes, l_train, is_sat
                    );

                    let filename = format!(
                        "twotrack_train{:06}_line{:06}_station{:06}_n{:06}.json",
                        l_train, l_line_route, l_station_route, n_line_routes
                    );

                    ::serde_json::to_writer(&std::fs::File::create(filename).unwrap(), &problem).unwrap();

                    //let solution = solve_3_using_local_and_global_progress(&problem, &|_| {});
                    //match solution {
                    //    crate::plan::DeadlockResult::Live(_) => {
                    //        assert!(is_sat);
                    //    }
                    //    crate::plan::DeadlockResult::Deadlocked(_) => {
                    //        assert!(!is_sat);
                    //    }
                    //}
                }
            }
        }
    }

    #[test]
    pub fn two_track_multi() {
        let l_line_route = 100;
        for l_station_route in [5] {
            for n_stations in [2,4,6,8,10,20,50,100] {
                for l_train in [10] {
                    let is_sat = l_train <= l_station_route;
                    let problem = generate_multistation_twotrack(l_line_route, l_station_route, n_stations, l_train);

                    assert!(!is_sat);

                    println!(
                        "two_track_multi l_station {} n_stations {} l_train {} is_sat {}",
                        l_station_route, n_stations, l_train, is_sat
                    );

                    let filename = format!(
                        "twotrack_multi_train{:06}_line{:06}_station{:06}_n{:06}.json",
                        l_train, l_line_route, l_station_route, n_stations
                    );

                    ::serde_json::to_writer(&std::fs::File::create(filename).unwrap(), &problem).unwrap();

                    //let solution = solve_3_using_local_and_global_progress(&problem, &|_| {});
                    //match solution {
                    //    crate::plan::DeadlockResult::Live(_) => {
                    //        assert!(is_sat);
                    //    }
                    //    crate::plan::DeadlockResult::Deadlocked(_) => {
                    //        assert!(!is_sat);
                    //    }
                    //}
                }
            }
        }
    }
}

#[allow(dead_code)]
pub fn generate_two_track_instance(
    l_line_route: u64,
    l_station_route: u64,
    n_line_routes: u64,
    l_train: u64,
) -> Problem {
    let mut t1_routes = HashMap::new();
    let mut t2_routes = HashMap::new();

    // before t1
    for (this, other, routes) in [("t1", "t2", &mut t1_routes), ("t2", "t1", &mut t2_routes)] {
        // LINE
        for i in 1..=n_line_routes {
            routes.insert(
                format!("{}_r{}", this, i),
                TrainRoute {
                    train_length: l_train,
                    route_length: l_line_route,
                    switch_length: 1,
                    unconditional_conflicts: vec![format!("{}_r{}", other, n_line_routes - i)],
                    allocation_conflicts: vec![],
                    next_routes: if i == n_line_routes {
                        None
                    } else if i == n_line_routes / 2 {
                        Some(vec![format!("{}_station_a", this), format!("{}_station_b", this)])
                    } else {
                        Some(vec![format!("{}_r{}", this, i + 1)])
                    },
                },
            );
        }

        // STATION
        for ab in ["a", "b"] {
            routes.insert(
                format!("{}_station_{}", this, ab),
                TrainRoute {
                    train_length: l_train,
                    route_length: l_station_route,
                    switch_length: 1,
                    unconditional_conflicts: vec![format!("{}_station_{}", other, ab)],
                    allocation_conflicts: vec![],
                    next_routes: Some(vec![format!("{}_r{}", this, n_line_routes / 2 + 1)]),
                },
            );
        }
    }

    let train1 = Train {
        name: "Train 1".to_string(),
        initial_routes: vec!["t1_r1".to_string()],
        routes: t1_routes,
    };

    let train2 = Train {
        name: "Train 2".to_string(),
        initial_routes: vec!["t2_r1".to_string()],
        routes: t2_routes,
    };

    Problem {
        trains: vec![train1, train2],
    }
}

#[allow(dead_code)]
pub fn generate_multistation_twotrack(
    l_line_route: u64,
    l_station_route: u64,
    n_stations: u64,
    l_train: u64,
) -> Problem {
    let mut t1_routes = HashMap::new();
    let mut t2_routes = HashMap::new();

    // before t1
    for (this, other, routes) in [("t1", "t2", &mut t1_routes), ("t2", "t1", &mut t2_routes)] {
        for i in 0..n_stations {
            // LINE IN
            routes.insert(
                format!("{}_line{}in", this, i),
                TrainRoute {
                    train_length: l_train,
                    route_length: l_line_route,
                    switch_length: 1,
                    unconditional_conflicts: vec![format!("{}_line{}out", other, n_stations - 1 - i)],
                    allocation_conflicts: vec![],
                    next_routes: Some(vec![
                        format!("{}_station{}_a", this, i),
                        format!("{}_station{}_b", this, i),
                    ]),
                },
            );

            // STATION
            for ab in ["a", "b"] {
                routes.insert(
                    format!("{}_station{}_{}", this, i, ab),
                    TrainRoute {
                        train_length: l_train,
                        route_length: l_station_route,
                        switch_length: 1,
                        unconditional_conflicts: vec![format!("{}_station{}_{}", other, n_stations - 1 - i, ab)],
                        allocation_conflicts: vec![],
                        next_routes: Some(vec![format!("{}_line{}out", this, i)]),
                    },
                );
            }

            // LINE OUT
            routes.insert(
                format!("{}_line{}out", this, i),
                TrainRoute {
                    train_length: l_train,
                    route_length: l_line_route,
                    switch_length: 1,
                    unconditional_conflicts: vec![format!("{}_line{}in", other, n_stations - 1 - i)],
                    allocation_conflicts: vec![],
                    next_routes: (i < (n_stations-1)).then(|| vec![format!("{}_line{}in", this, i + 1)]),
                },
            );
        }
    }

    let train1 = Train {
        name: "Train 1".to_string(),
        initial_routes: vec!["t1_line0in".to_string()],
        routes: t1_routes,
    };

    let train2 = Train {
        name: "Train 2".to_string(),
        initial_routes: vec!["t2_line0in".to_string()],
        routes: t2_routes,
    };

    Problem {
        trains: vec![train1, train2],
    }
}

pub fn to_dot(problem: &Problem) -> String {
    let mut lines = Vec::new();
    let colors = vec!["blue", "green", "orange", "teal", "pink", "brown", "black", "purple"];

    let mut conflicts = Vec::new();
    let mut routes = std::collections::HashSet::new();

    for (train_idx, train) in problem.trains.iter().enumerate() {
        let forward = train_idx == 0;
        assert!(train_idx < colors.len());
        lines.push(format!("i{} [shape=plaintext]", train.name));
        for initial in train.initial_routes.iter() {
            if forward {
                lines.push(format!("i{} -> r{} []", train.name, initial));
            } else {
                lines.push(format!("r{} -> i{} [dir=back]", initial, train.name));
            }
        }
        for (route, routedata) in train.routes.iter() {
            routes.insert(route);
            if let Some(next) = routedata.next_routes.as_ref() {
                for next in next.iter() {
                    if forward {
                        lines.push(format!("r{} -> r{} [color={}]", route, next, colors[train_idx]));
                    } else {
                        lines.push(format!(
                            "r{} -> r{} [color={},dir=back]",
                            next, route, colors[train_idx]
                        ));
                    }
                }
            } else {
                lines.push(format!("out{} [shape=plaintext]", route));
                if forward {
                    lines.push(format!("r{} -> out{} [color={}]", route, route, colors[train_idx]));
                } else {
                    lines.push(format!(
                        "out{} -> r{} [color={},dir=back]",
                        route, route, colors[train_idx]
                    ));
                }
            }

            for c in routedata.unconditional_conflicts.iter() {
                if route < c {
                    conflicts.push((route, c));
                }
            }
        }
    }

    for (a, b) in conflicts {
        if routes.contains(&a) && routes.contains(&b) {
            lines.push(format!(
                "r{} -> r{} [color=red, arrowhead=none, style=dashed, constraint=false]",
                a, b
            ));
        }
    }

    return format!(
        "
        digraph deadlockinstance {{
            splines = false
            {}
        }}
    ",
        lines.join("\n")
    );
}

use crate::raw_problem;
pub fn parse(problem: &raw_problem::Problem) -> Problem {
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

            let switch_length = conflict_sets[1].length - conflict_sets[0].length;
            assert!(switch_length == 1);

            routes.insert(
                trainroute.route.clone(),
                TrainRoute {
                    route_length: conflict_sets[1].length,
                    train_length: trainroute.length,
                    switch_length,
                    unconditional_conflicts,
                    allocation_conflicts,
                    next_routes: (!trainroute.is_black_hole).then(|| trainroute.next_routes.clone()),
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
        summary.push_str(&format!("t{}   @{}", train.name, train.initial_routes.join(", ")));
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

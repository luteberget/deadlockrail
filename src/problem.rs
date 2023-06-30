use crate::raw2021_problem;
use crate::raw2023_problem;
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

pub fn convert_raw2023(problem: &raw2023_problem::Problem) -> Problem {


    // Let's check a few assumptions first.
    
    // Are there more than two conflict sets (different 
    // releases at different lengths) ?

    for train in problem.trains.iter() {
        if train.is_dummy {
            trace!("Skipping dummy train {}/{}", train.id, train.name);
            continue;
        }
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
                println!("conflict sets:, {:?}", conflict_sets);
                panic!("Conflicts should always be listed in two length levels.");
            }
        }
    }

    todo!()
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

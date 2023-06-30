use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Problem {
    pub id: String,
    pub mode: String,
    pub trains: Vec<Train>,
    pub routes: Vec<Route>,
    pub train_routes: Vec<TrainRoute>,
    pub conflicts: Vec<RouteLengthExclusion>,
    pub safeplace_alt_path_set: Vec<SafeplaceAltPathSet>,
    pub train_exit_sequence_set: Vec<TrainExitSequenceSet>,
}

#[derive(Deserialize, Debug)]
pub struct Train {
    pub name: String,
    pub id: String,
    pub is_dummy: bool,
    pub initial_routes: Vec<String>,
    pub final_routes: Vec<String>,
    pub crossing_trains: Vec<String>,
    pub follower_trains: Vec<String>,
    pub always_win_after_first_win: Vec<String>,
    pub is_safe_place_bound: bool,
    pub safe_place_route: Vec<String>,
    pub initial_multitrain_prev_trainid: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct Route {
    pub name: String,
    pub id: String,

    pub is_multi_train: bool,

    pub final_point_id: String,
    pub final_point_container_id: String,
    pub is_final_point_in_station: bool,

    pub is_siding: bool,
    pub is_unusable: bool,
    pub is_no_meet_pass: bool,
    pub is_only_track_blocked: bool,
}

#[derive(Deserialize, Debug)]
pub struct TrainRoute {
    pub train: String,
    pub route: String,
    pub length: u64,
    pub is_potential_safe_place: bool,
    pub is_black_hole: bool,
    pub next_routes: Vec<String>,
    pub time_in_opt: String,
}

#[derive(Deserialize, Debug)]
pub struct RouteLengthExclusion {
    pub route: String,
    pub length: u64,
    pub conflicts: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct TrainExitSequenceSet {
    pub black_hole_route_id: String,
    pub train_id_exit_sequence: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct SafeplaceAltPathSet {
    pub group_id: String,
    pub route_id_sequence: Vec<String>,
}

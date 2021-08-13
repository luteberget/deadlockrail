use serde::Deserialize;

#[derive(Deserialize)]
pub struct Problem {
    pub trains: Vec<Train>,
    pub routes: Vec<Route>,
    pub train_routes: Vec<TrainRoute>,
    pub conflicts: Vec<RouteLengthExclusion>,
}

#[derive(Deserialize)]
pub struct Train {
    pub name: String,
    pub id: String,
    pub is_dummy: bool,
    pub initial_routes: Vec<String>,
    pub final_routes: Vec<String>,
    pub crossing_trains: Vec<String>,
    pub follower_trains: Vec<String>,
    pub is_safe_place_bound: bool,
    pub safe_place_route: String,
}

#[derive(Deserialize)]
pub struct Route {
    pub name: String,
    pub id: String,

    /// an more than one trani be simultaneously on the route?
    /// (TODO: what does this mean)
    /// NOTE Always false
    pub is_multi_train: bool,

    /// Id of the statino or the track where the end point of the route is placed.
    pub station_or_track_id: String,
    /// the end point of the route belongs to a station
    pub is_final_point_in_station: bool,

    /// If the station route is a siding
    /// NOTE Always false
    pub is_siding: bool,

    /// If for some reason the route cannot be used by any train
    /// NOTE Always false
    pub is_unusable: bool,
}

#[derive(Deserialize)]
pub struct TrainRoute {
    pub train: String,
    pub route: String,
    pub length: u64,
    pub is_potential_safe_place: bool,
    pub is_black_hole: bool,
    pub next_routes: Vec<String>,
}

#[derive(Deserialize)]
pub struct RouteLengthExclusion {
    pub route: String,
    pub length: u64,
    pub conflicts: Vec<String>,
}

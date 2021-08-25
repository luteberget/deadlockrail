import csv
import json

def parse_bool(s):
    assert(s == "true" or s == "false")
    return s == "true"

def parse_sep(s):
    if s == "":
        return []
    return list(s.split(","))

for i in range(1,20+1):
    instance = {}
    prefix = f"original_files/Instance{i}_"
    with open(f"{prefix}RawTrainSet.tab") as tsv:
        r = csv.reader(tsv, dialect="excel-tab")
        header = next(r)
        #print(f"header: {header}")
        h1 = ['trainStr', 'trainId', 'isDummy', 'initialRouteIdsCsv', 'finalRouteIdsCsv', 'crossingTrainIdsCsv', 'followerTrainIdsCsv', 'isSafePlaceBound', 'safePlaceRoute']
        h2 = ['trainIdStr', 'trainId', 'isDummy', 'initialRouteIdsCsv', 'lastRouteIdsCsv', 'crossingTrainIdsCsv', 'followerTrainIdsCsv', 'isSafePlaceBound', 'safePlaceRoute']
        h3 = ['trainStr', 'trainId', 'isDummy', 'initialRouteIdsCsv', 'finalRoute', 'crossingTrainIdsCsv', 'followerTrainIdsCsv', 'isSafePlaceBound', 'safePlaceRoute']
        assert(header == h1 or header == h2 or header == h3)
        instance["trains"] = \
                [{ "name": line[0], 
                    "id": line[1],
                    "is_dummy": parse_bool(line[2]), 
                    "initial_routes": parse_sep(line[3]),
                    "final_routes": parse_sep(line[4]),
                    "crossing_trains": parse_sep(line[5]),
                    "follower_trains": parse_sep(line[6]),
                    "is_safe_place_bound": parse_bool(line[7]),
                    "safe_place_route": line[8],
                    } 
                    for line in r]

    with open(f"{prefix}RawRouteSet.tab") as tsv:
        r = csv.reader(tsv, dialect="excel-tab")
        header = next(r)
        #print(f"header: {header}")
        h1 = ['RouteStr', 'routeId', 'isMultiTrain', 'stationOrTrackId', 'isFinalPointInStation', 'isSiding', 'isUnusable']
        h2 = ['routeStr', 'routeId', 'isMultiTrain', 'stationOrTrackId', 'isFinalPointInStation', 'isSiding', 'isUnusuable']
        h3 = ['railwayRouteId', 'routeId', 'isMultiTrain', 'stationOrTrackId', 'isFinalPointInStation', 'isSiding', 'isUnusable']
        assert(header == h1 or header == h2 or header == h3)

        instance["routes"] = \
                [{ "name": line[0], 
                    "id":line[1], 
                    "is_multi_train": parse_bool(line[2]),
                    "station_or_track_id": line[3], 
                    "is_final_point_in_station": parse_bool(line[4]), 
                    "is_siding": parse_bool(line[5]), 
                    "is_unusable": parse_bool(line[6])}
                 for line in r]

    with open(f"{prefix}RawTrainRouteSet.tab") as tsv:
        r = csv.reader(tsv, dialect="excel-tab")
        header = next(r)
        #print(f"header: {header}")
        h1 = ['trainId', 'routeId', 'trainLength', 'isPotentialSafePlace', 'isBlackHole', 'nextRouteIdCsv']
        assert(header == h1)
        instance["train_routes"] = \
                [{ "train": line[0],
                    "route": line[1],
                    "length": int(line[2]),
                    "is_potential_safe_place": parse_bool(line[3]),
                    "is_black_hole": parse_bool(line[4]),
                    "next_routes": parse_sep(line[5]) }
                for line in r]

    with open(f"{prefix}RawRouteIncompByLenSet.tab") as tsv:
        r = csv.reader(tsv, dialect="excel-tab")
        header = next(r)
        #print(f"header: {header}")
        h1 = ['routeId', 'length', 'incompRouteIdsCsv']
        assert(header == h1)
        instance["conflicts"] = \
                [{ "route": line[0],
                    "length": int(line[1]),
                    "conflicts":parse_sep(line[2]) }
                for line in r]


    with open(f'instance{i}.json', 'w') as f:
        json.dump(instance, f, indent=2)

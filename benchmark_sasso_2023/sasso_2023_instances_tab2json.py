import csv
import json
import glob

def parse_bool(s):
    assert(s == "true" or s == "false")
    return s == "true"

def parse_sep(s):
    if s == "":
        return []
    return [x.strip() for x in s.split(",")]

instances = sorted([ x.removesuffix("_RawInfo.tab") 
              for x in glob.glob("original_files/*_RawInfo.tab") ])

print(f"Parsing {len(instances)} problem instances")

for prefix in instances:
    instancename = prefix.removeprefix("original_files/")
    print(f"Converting instance '{prefix}'")
    instance = {}

    with open(f"{prefix}_RawInfo.tab") as tsv:
        r = csv.reader(tsv, dialect="excel-tab")
        id_k, id_v = next(r)
        mode_k, mode_v = next(r)
        assert(id_k == "Id")
        assert(mode_k == "Mode")
        instance["id"] = id_v
        instance["mode"] = mode_v

    with open(f"{prefix}_RawTrainSet.tab") as tsv:
        r = csv.reader(tsv, dialect="excel-tab")
        header = next(r)
        #print(f"header: {header}")
        h = ['trainIdStr','trainId','isDummy','initialRouteIdsCsv','lastRouteIdsCsv','crossingTrainIdsCsv','followerTrainIdsCsv','alwaysWinAfterFirstWinTrainIdsCsv','isSafePlaceBound','safePlaceRouteIdOpt','initialMultitrainPrevTrainIdOpt']
        assert(header == h)
        r = [l for l in r]
        assert(all(len(header) == len(l) for l in r))
        instance["trains"] = \
                [{ "name": line[0].strip(), 
                    "id": line[1].strip(),
                    "is_dummy": parse_bool(line[2]), 
                    "initial_routes": parse_sep(line[3]),
                    "final_routes": parse_sep(line[4]),
                    "crossing_trains": parse_sep(line[5]),
                    "follower_trains": parse_sep(line[6]),
                    "always_win_after_first_win" : parse_sep(line[7]),
                    "is_safe_place_bound": parse_bool(line[8]),
                    "safe_place_route": parse_sep(line[9]),
                    "initial_multitrain_prev_trainid": parse_sep(line[10]),
                    } 
                    for line in r]

    with open(f"{prefix}_RawRouteSet.tab") as tsv:
        r = csv.reader(tsv, dialect="excel-tab")
        header = next(r)
        #print(f"header: {header}")
        h = ['railwayRouteId', 'routeId', 'isMultiTrain', 
                'finalPointId','finalPointContainerId','isFinalPointInStation',
                'isSiding','isUnusable','isNoMeetPass','isOnlyTrackBlocked']
        assert(header == h)
        r = [l for l in r]
        assert(all( len(header) == len(l) for l in r))

        instance["routes"] = \
                [{ "name": line[0].strip(), 
                    "id":line[1].strip(), 
                    "is_multi_train": parse_bool(line[2]),
                    "final_point_id": line[3].strip(),
                    "final_point_container_id": line[4].strip(),
                    "is_final_point_in_station": parse_bool(line[5]), 
                    "is_siding": parse_bool(line[6]), 
                    "is_unusable": parse_bool(line[7]),
                    "is_no_meet_pass": parse_bool(line[8]),
                    "is_only_track_blocked": parse_bool(line[9])}
                 for line in r]


    with open(f"{prefix}_RawTrainRouteSet.tab") as tsv:
        r = csv.reader(tsv, dialect="excel-tab")
        header = next(r)
        #print(f"header: {header}")
        h1 = ['trainId', 'routeId', 'trainLength', 'isPotentialSafePlace', 'isBlackHole', 'nextRouteIdCsv', 'timeInOpt']
        assert(header == h1)
        instance["train_routes"] = \
                [{ "train": line[0].strip(),
                    "route": line[1].strip(),
                    "length": int(line[2].strip()),
                    "is_potential_safe_place": parse_bool(line[3]),
                    "is_black_hole": parse_bool(line[4]),
                    "next_routes": parse_sep(line[5]),
                    "time_in_opt": line[6].strip()}
                for line in r]

    with open(f"{prefix}_RawRouteIncompByLenSet.tab") as tsv:
        r = csv.reader(tsv, dialect="excel-tab")
        header = next(r)
        #print(f"header: {header}")
        h1 = ['routeId', 'length', 'incompRouteIdsCsv']
        assert(header == h1)
        instance["conflicts"] = \
                [{ "route": line[0].strip(),
                    "length": int(line[1].strip()),
                    "conflicts":parse_sep(line[2]) }
                for line in r]

    with open(f"{prefix}_RawTrainExitSequenceSet.tab") as tsv:
        r = csv.reader(tsv, dialect="excel-tab")
        header = next(r)
        #print(f"header: {header}")
        h1 = ['blackholeRouteId','trainIdExitSequenceCsv']
        assert(header == h1)
        instance["train_exit_sequence_set"] = \
                [{ "black_hole_route_id": line[0].strip(),
                    "train_id_exit_sequence":parse_sep(line[1]) }
                for line in r]

    with open(f"{prefix}_RawSafeplaceAltPathSet.tab") as tsv:
        r = csv.reader(tsv, dialect="excel-tab")
        header = next(r)
        #print(f"header: {header}")
        h1 = ['groupId','routeIdSequence']
        assert(header == h1)
        instance["safeplace_alt_path_set"] = \
                [{ "group_id": line[0].strip(),
                   "route_id_sequence":parse_sep(line[1]) }
                for line in r]

    with open(f'instance_{instancename}.json', 'w') as f:
        json.dump(instance, f, indent=2)

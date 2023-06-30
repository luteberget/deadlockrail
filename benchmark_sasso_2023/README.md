This directory contains the benchmark instances used in an in-progress work (as
of 2023-06-30) by Veronica Dal Sasso, Leonardo Lamorgese, Carlo Mannino, Andrea
Onofri, and Paolo Ventura, on deadlock detection using cycle detection and 0-1
linear programming.
	
The original instance files supplied by Veronica Dal Sasso are located in the
directory `original_files`, in tabular format, and a description of the tabular
format can be found below, in this file.  The script
`sasso_2023_instances_tab2json.py` converts the set of tabular files corresponding
to an instance into a single JSON file.

# Tabular format description

- Instance RawTrainSet: here you find the set of trains (one per each row).
  Note: we do not have dummy trains any more. For each train you have the
  identificative, the route(s) where it is initially placed, the last route (if
  the train has its final destination within the considered area. I think it is
  never set in these instances), the IDs of crossing and trailing trains, the
  ID of trains which have higher priority (that is, trains that cannot be
  bypassed by the train), if the train has to stop in safe place and which is
  the dedicated safe place (also these two fields should be never filled in
  these instances). Last, if there is more than one train on the same initial
  resource, the ID(s) of the trains in front.

- Instance RawRouteSet: here you find the set of routes. For each route you
  have: the ID, isMultiTrain=TRUE if more than one train can be simultaneously
  on that route; finalPointID = the identificative of the control point at the
  end of the route, finalPointContainerId = the identificative of the station
  or track where the end point of the route is placed,
  isFinalPointInStation=TRUE if the end point of the route belongs to a
  station,; isSiding=TRUE if the station route is a siding; isUnusable=TRUE if
  for some reason the route cannot be used by any train, isNoMeetPass (you can
  ignore this); isOnlyTrackBlocked = TRUE if isUnusable=true but the block does
  not cover also the switch.

- Instance RawTrainRouteSet: here you find, for each train, its feasible routes
  and the train's length when it traverses such route. For each route,
  isPotentialSafePlace=FALSE if it is trivial to see that the route cannot be
  used to park the train in safe place (e.g., if the route is a single track in
  open line); isBlackHole=TRUE means that the route is a dummy one, that is
  used to link the real routes to the external network; nextRouteIdCsv records
  the routes that can be reached from the route itself. timeInOpt should always
  be empty

- Instance RawRouteIncompByLenSet: here you find route incompatibilities. Notice
  that, for each route, you find 2 rows, each one associated to a different
  route length. In fact, routes are made of either a stopping point or a track
  plus the entry/exit routes to/from the station. If a train, once stopped at a
  signal, occupies only the stopping point or a track (which length is the
  shorter one associated to the route), then it is incompatible only with those
  routes ending on the same track/stopping point. Otherwise, if the train's
  length is greater than the shorter length associated with the route, it also
  impedes the switch at the entry/exit route, hence it is incompatible also
  with those routes that share the switch. 

- Instance RawSafeplaceAltPathSet: this is a completely new file. It is
  necessary to handle the new definition of safeplaces. That is: for each set
  of alternative paths that fully contains the safeplace location, there must
  be a free alternative path. Each row of this file has the id of the
  alternative paths set in the first column and a sequence of routes in the
  second column.



This directory contains the benchmark instances from the following paper: 

 > Veronica Dal Sasso, Leonardo Lamorgese, Carlo Mannino, Andrea Onofri & Paolo Ventura (2021):The Tick Formulation for deadlock detection and avoidance in railways traffic control. J. Rail Transp. Plan. Manag.17,p. 100239, doi:10.1016/j.jrtpm.2021.100239.

The original instance files supplied by Veronica Dal Sasso are located in the
directory `original_files`, in tabular format, and a description of the tabular
format can be found below, in this file.  The script
`sasso_instances_tab2json.py` converts the set of tabular files corresponding
to an instance into a single JSON file.


# Tabular format description

- RawTrainSet: here you find the set of trains (one per each row). You can see
  that in each instance there are at the bottom a couple of trains which have
  TRUE under the isDummy flag. These are not part of the instance, they are
  just dummy trains we use to identify safe places. For each train you have the
  identificative, the route(s) where it is initially placed, the last route (if
  the train has its final destination within the considered area. I think it is
  never set in these instances), the IDs of crossing and trailing trains, if
  the train has to stop in safe place and which is the dedicated safe place
  (also these two fields should be never filled in these instances)

- RawRouteSet: here you find the set of routes. The double ID is irrelevant
  here, and it is the same. Just consider one of the two. Then, for each route
  you have: isMultiTrain=TRUE if more than one train can be simultaneously on
  that route; stationOrTrackId the identificative of the station or track where
  the end point of the route is placed, isFinalPointInStation=TRUE if the end
  point of the route belongs to a station,; isSiding=TRUE if the station route
  is a siding; isUnusable=TRUE if for some reason the route cannot be used by
  any train 

- RawTrainRouteSet: here you find, for each train, its feasible routes and the
  train's length when it traverses such route. For each route,
  isPotentialSafePlace=FALSE if it is trivial to see that the route cannot be
  used to park the train in safe place (e.g., if the route is a single track in
  open line); isBlackHole=TRUE means that the route is a dummy one, that is
  used to link the real routes to the external network; nextRouteIdCsv records
  the routes that can be reached from the route itself.

- RawRouteIncompByLenSet: here you find route incompatibilities. Notice that,
  for each route, you find 2 rows, each one associated to a different route
  length. In fact, routes are made of either a stopping point or a track plus
  the entry/exit routes to/from the station. If a train, once stopped at a
  signal, occupies only the stopping point or a track (which length is the
  shorter one associated to the route), then it is incompatible only with those
  routes ending on the same track/stopping point. Otherwise, if the train's
  length is greater than the shorter length associated with the route, it also
  impedes the switch at the entry/exit route, hence it is incompatible also
  with those routes that share the switch. 



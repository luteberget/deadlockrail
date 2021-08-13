use std::collections::{HashMap, HashSet};

// use good_lp::{Solution, SolverModel, constraint, variable};
use log::warn;
use velcro::iter;

use crate::problem;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Coord {
    x: u32,
    y: u32
}

#[derive(serde::Serialize, serde::Deserialize)]

pub struct RoutePlot {
    pub routes :HashMap<String, Coord>,
    pub dir :Vec<bool>,
}

impl RoutePlot {

    #[allow(dead_code)]
    pub fn plot_string(&self, plan :&crate::plan::Plan) -> String {
        let _p = hprof::enter("plot_string");
        plan.steps.iter().enumerate().map(|(step_idx, step)| {
            // println!("plotting step {}", step_idx);
            let width = 12;
            let nx = self.routes.values().map(|c| c.x).max().unwrap() + 1;
            let ny = self.routes.values().map(|c| c.y).max().unwrap() + 1;
            let mut lines = (0..(2*ny+1)).map(|_l| vec![b' '; (width+1)*nx as usize + 1]).collect::<Vec<_>>();
            // println!("lines len {}, inside {}", lines.len(), lines[0].len());
            let mut write = |x :u32,y :u32, b| {
                // println!("plotting at {} {}", x,y );
                let y = lines.len() - 1 - y as usize;
                lines[y][x as usize] = b;
            };

            for coord in self.routes.values() {
                // println!("coord ({},{})", coord.x, coord.y);
                for y in [2*coord.y, 2*(coord.y+1)] {
                    for x in ((width+1)*coord.x as usize)..((width+1)*(coord.x as usize+1)) {
                        write(x as u32, y as u32, b'-');
                    }
                }
                for x in [((width+1)*coord.x as usize),((width+1)*(coord.x as usize+1))] {
                    for y in (2*coord.y)..=(2*(coord.y+1)) {
                        let c = if y == 2*coord.y || y == 2*(coord.y+1) { b'+'} else {b'|'};
                        write(x as u32, y as u32, c);
                    }
                }
            }

            let mut taken = HashMap::new();

            for (route, train) in step.iter().filter_map(|(r,t)| t.map(|t| (r,t))) {
                let mut string = if !self.dir[train] {
                    format!("{}@{}->", train,route)
                } else {
                    format!("<-{}@{}", train,route)
                };
                
                string.truncate(width);
                let coord = &self.routes[route];
                let coordtuple = (coord.x,coord.y);
                 println!("t{} r{} at {:?}", train, route, coordtuple);
                let taken_condition = !taken.contains_key(&coordtuple) || taken[&coordtuple] == train;
                if !taken_condition {
                    warn!("Shouldn't plot train {} route {} at {:?} because train {} is already there", train, route, coordtuple, taken[&coordtuple]);
                    continue;
                }
                taken.insert(coordtuple,train);
                let (x,y) = ((width+1)*coord.x as usize +1, 2*coord.y + 1); 

                for (dx,c) in string.bytes().enumerate() {
                    write(x as u32 + dx as u32,y,c);
                }
            }
        
            lines.insert(0, format!("Step {}", step_idx).as_bytes().to_vec());
            lines.into_iter().map(|l| String::from_utf8(l).unwrap()).collect::<Vec<_>>().join("\n")
        }).collect::<Vec<_>>().join("\n")
    }

    #[allow(dead_code)]
    pub fn plot_string2(&self, plan :&crate::plan::Plan) -> String {
        let _p = hprof::enter("plot_string");
        plan.steps.iter().enumerate().map(|(step_idx, step)| {

            let nx = self.routes.values().map(|c| c.x).max().unwrap() + 1;
            let mut station = (0..nx).map(|_| Vec::new()).collect::<Vec<_>>();            

            for (route, train) in step.iter().filter_map(|(r,t)| t.map(|t| (r,t))) {
                let string = if !self.dir[train] {
                    format!("{}@{}->", train,route)
                } else {
                    format!("<-{}@{}", train,route)
                };
                
                let coord = &self.routes[route];
                station[coord.x as usize].push((train,string));
            }

            let ny = step.iter().filter_map(|(_r,t)| t.map(|t| t)).max().unwrap() as usize + 1;
            let width = 12;
            let mut lines = (0..ny).map(|_l| vec![b' '; (width+1)*nx as usize + 1]).collect::<Vec<_>>();

            for x in 0..nx {
                for line in lines.iter_mut() {
                    line[(width+1)*x as usize] = b'|';
                }
            }
            
            for (x,strings) in station.iter().enumerate() {
                for (y,s) in strings.iter() {
                    for (dx,c) in s[..(s.len().min(width))].bytes().enumerate() {
                        lines[*y][(width+1)*x as usize + 1 + dx] = c;
                    }
                }
            }

            lines.insert(0, format!("Step {}", step_idx).as_bytes().to_vec());
            lines.into_iter().map(|l| String::from_utf8(l).unwrap()).collect::<Vec<_>>().join("\n")
        }).collect::<Vec<_>>().join("\n")
    }
}

pub fn solve(problem :&problem::Problem) -> RoutePlot {
    todo!()
    // let mut vars = good_lp::variables!();
    // let mut cs = Vec::new();

    // let routes = problem.trains.iter().flat_map(|r| r.routes.keys()).collect::<HashSet<&String>>();
    // let route_xs = routes.iter()
    //     .map(|r| (r, vars.add(variable().integer().min(0))))
    //     .collect::<HashMap<_,_>>();
    // let route_ys = routes.iter()
    //     .map(|r| (r, vars.add(variable().integer().min(0))))
    //     .collect::<HashMap<_,_>>();

    // let mut obj = good_lp::Expression::from(0.);
    // for (_,v) in route_xs.iter() {
    //     obj += v;
    // }
    // for (_,v) in route_ys.iter() {
    //     obj += v;
    // }
    

    // // trains go in one direction and then routes are consecutive
    
    // #[allow(non_snake_case)]
    // let M = route_xs.len() as f64;
    
    // let mut train_dir_vars = Vec::new();
    // for train in problem.trains.iter() {
    //     let going_up = vars.add(variable().integer().min(0).max(1));
    //     train_dir_vars.push(going_up);
    //     for (route, routedata) in train.routes.iter() {
    //         if let Some(nexts) = routedata.next_routes.as_ref() {
    //             for next in nexts.iter() {
    //                 cs.push(constraint!(   route_xs[&route]  + 1 <= route_xs[&next] + M*going_up   ));
    //                 cs.push(constraint!(   route_xs[&route]  - 1 >= route_xs[&next] - M*(1 - going_up)  ));
    //             }

    //             for next1 in 0..nexts.len() {
    //                 for next2 in (next1+1)..nexts.len() {
    //                     let alt = vars.add(variable().integer().min(0).max(1));
    //                     cs.push(constraint!(  route_ys[&&nexts[next1]] +1 <= route_ys[&&nexts[next2]] + M*alt ));
    //                     cs.push(constraint!(  route_ys[&&nexts[next1]] -1 >= route_ys[&&nexts[next2]] - M*(1-alt) ));
    //                 }
    //             }
    //         }

    //         for other in iter![..&routedata.unconditional_conflicts, ..&routedata.allocation_conflicts] {
    //             if !route_xs.contains_key(&other) {
    //                 println!("no route for {}", other);
    //                 continue;
    //             }

                
    //             let distance_x = vars.add(variable().min(0));
    //             cs.push(constraint!(  distance_x >= (  route_xs[&other] - route_xs[&route]  )   ));
    //             cs.push(constraint!(  distance_x >= (  route_xs[&route] - route_xs[&other]  )   ));
    //             obj += 100*distance_x;

                
    //             let distance_y = vars.add(variable().min(0));
    //             cs.push(constraint!(  distance_y >= (  route_ys[&other] - route_ys[&route]  )   ));
    //             cs.push(constraint!(  distance_y >= (  route_ys[&route] - route_ys[&other]  )   ));
    //             obj += 100*distance_y;

    //             //cs.push(constraint!( distance_y + distance_x >= 1));

                

    //             // Correct, but slow:
    //             // let xy = vars.add(variable().integer().min(0).max(1));
    //             // let dir = vars.add(variable().integer().min(0).max(1));
    //             // cs.push(constraint!( route_xs[&route] + 1 <= route_xs[&other] + M*dir + M*xy));
    //             // cs.push(constraint!( route_xs[&route] - 1 >= route_xs[&other] - M*(1 - dir) - M*xy));
    //             // cs.push(constraint!( route_ys[&route] + 1 <= route_ys[&other] + M*dir + M*(1-xy)));
    //             // cs.push(constraint!( route_ys[&route] - 1 >= route_ys[&other] - M*(1 - dir) - M*(1-xy)));

    //         }
    //     }
    // }


    // let mut lp = vars
    //     .minimise(obj)
    //     .using(good_lp::default_solver);

    // for c in cs { lp.add_constraint(c);}

    // println!("solving");
    // let solution = lp
    //     .solve()
    //     .unwrap();
    // println!("solving done");

    // let mut route_coords = HashMap::new();
    // for route in routes.iter() {
    //     let x = solution.eval(route_xs[route]).round() as u32;
    //     let y = solution.eval(route_ys[route]).round() as u32;
    //     println!("r = {}, x = {}, y = {}", route, x, y);

    //     route_coords.insert((*route).clone(), Coord {x, y});
    // }

    // let mut dirs = Vec::new();
    // for (train,dir) in problem.trains.iter().zip(train_dir_vars.iter()) {
    //     println!("Train {} dir {}", train.name, if solution.eval(dir) == 0.0 { "up"} else { "down"});
    //     dirs.push(solution.eval(dir) != 0.0);
    // }


    // RoutePlot{routes: route_coords, dir: dirs}
}
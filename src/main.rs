use log::*;

mod plan;
mod plot;
mod problem;
mod raw_problem;
mod solver;
mod state;

use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "deadlock", about = "Railway deadlock checker.")]
struct Opt {
    /// Problem instance
    #[structopt(name = "FILE")]
    #[structopt(parse(from_os_str))]
    file: PathBuf,

    /// Write DOT file for visualizing the problem instance
    #[structopt(short)]
    #[structopt(parse(from_os_str))]
    dotoutput: Option<PathBuf>,

    /// Solve and save plot data to file.
    #[structopt(long)]
    #[structopt(parse(from_os_str))]
    save_plot_data: Option<PathBuf>,

    /// Load plot data from file.
    #[structopt(long)]
    #[structopt(parse(from_os_str))]
    load_plot_data: Option<PathBuf>,

    /// Write plan JSON if instance is not deadlocked.
    #[structopt(short)]
    #[structopt(parse(from_os_str))]
    planoutputfile: Option<PathBuf>,

    /// Activate debug mode
    // short and long flags (-d, --debug) will be deduced from the field's name
    #[structopt(short, long)]
    verbose: bool,

    #[structopt(long)]
    no_solve: bool,

    #[structopt(long)]
    trains_format: bool,

    #[structopt(long)]
    print_problem: bool,

    /// Choose between algorithms:
    /// 1 = algorithm simply adds states with consistency constraints and
    ///     solves until some upper bound of states is reached.
    /// 2 = extends 1 by adding a global progress constraints and solving
    ///     the system without the goal state assumption to check whether a deadlock
    ///     has been reached.
    /// 3 = extends 2 by adding local progress constraints which forces allocation
    ///     and freeing to happen as early as possible.
    #[structopt(long)]
    algorithm: Option<u8>,

    #[structopt(long)]
    introexample: bool,
}

fn main() {
    let _h1 = hprof::enter("init");

    let opt = Opt::from_args();
    let level = if opt.verbose {
        if cfg!(debug_assertions) {
            LevelFilter::Trace
        } else {
            LevelFilter::Info
        }
    } else {
        LevelFilter::Error
    };
    simple_logger::SimpleLogger::new().with_level(level).init().unwrap();
    info!("{:#?}", opt);
    drop(_h1);

    let problem = if opt.introexample {
        let mut p = problem::generate_multistation_twotrack(100, 100, 4, 150);
        let mut t3 = problem::Train {
            name: "t3".to_string(),
            initial_routes: vec![format!("t1_station2_b")],
            routes: p.trains[0].routes.clone(),
        };

        for (r, rs) in t3.routes.iter_mut() {
            rs.train_length = 50;
        }

        p.trains.push(t3);
        p
    } else {
        let json_contents = {
            let _h = hprof::enter("read file");
            trace!("Loading file {}", opt.file.to_str().unwrap());
            std::fs::read_to_string(&opt.file).unwrap()
        };

        let problem = {
            let _h = hprof::enter("parse");
            if !opt.trains_format {
                let problem = {
                    let raw_problem: raw_problem::Problem = serde_json::from_str(&json_contents).unwrap();
                    trace!(
                        "Converting problem with {} trains {} routes",
                        raw_problem.trains.len(),
                        raw_problem.routes.len()
                    );
                    problem::parse(&raw_problem)
                };
                problem
            } else {
                let problem: problem::Problem = serde_json::from_str(&json_contents).unwrap();
                problem
            }
        };
        problem
    };

    if let Some(f) = opt.dotoutput {
        let _h = hprof::enter("dot output");
        let dot = problem::to_dot(&problem);
        std::fs::write(f, dot).unwrap();
    }

    let mut plot_data = None;
    if let Some(f) = opt.save_plot_data {
        let _h = hprof::enter("plot data solve");
        let solution = plot::solve(&problem);
        std::fs::write(f, serde_json::to_string_pretty(&solution).unwrap()).unwrap();
        plot_data = Some(solution);
    }

    if let Some(f) = opt.load_plot_data {
        let _h = hprof::enter("plot data parse");
        trace!("Loading file {}", f.to_str().unwrap());
        let json_contents = std::fs::read_to_string(f).unwrap();
        let data: plot::RoutePlot = serde_json::from_str(&json_contents).unwrap();
        plot_data = Some(data);
    }

    if opt.print_problem {
        let _h = hprof::enter("pretty-printing problem");
        info!("PROBLEM: \n{:#?}", problem);
    } else {
        let num_routes: usize = problem.trains.iter().map(|t| t.routes.len()).sum();
        println!("NUM ROUTES/VARS: {}", num_routes);
    }

    if opt.no_solve {
        error!("Command-line options says not to solve. Exiting.");
        return;
    }
    {
        let _h = hprof::enter("deadlockcheck");

        let plt = &plot_data;
        let result = match opt.algorithm.unwrap_or(3) {
            1 => solver::solve_1_using_num_states_bound(&problem, &|p| {
                if let Some(d) = plt.as_ref() {
                    println!("{}", d.plot_string(p));
                }
            }),
            2 => solver::solve_2_using_global_progress(&problem, &|p| {
                if let Some(d) = plt.as_ref() {
                    println!("{}", d.plot_string(p));
                }
            }),
            3 => solver::solve_3_using_local_and_global_progress(&problem, &|p| {
                if let Some(d) = plt.as_ref() {
                    println!("{}", d.plot_string(p));
                }
            }),
            _ => panic!("Invalid algorithm {:?}", opt.algorithm),
        };

        match result {
            plan::DeadlockResult::Live(plan) => {
                info!("Plan found.");
                let plt = &plot_data;
                if let Some(d) = plt.as_ref() {
                    println!("{}", d.plot_string(&plan));
                } else {
                    let (summary, commands) = plan::print_plan(&plan);
                    debug!("Plan:\n{}", summary);
                    debug!("Commands:\n{}", commands);
                }
                if let Some(f) = opt.planoutputfile {
                    plan::write_plan_json(&f, plan).unwrap();
                    info!("Wrote plan to file {}", f.to_str().unwrap());
                }
            }
            plan::DeadlockResult::Deadlocked(_) => {
                // TODO print info
                info!("System is deadlocked.");
            }
        }
    }

    hprof::end_frame();
    hprof::profiler().print_timing();
}

use log::*;

mod plan;
mod problem;
mod raw2021_problem;
mod raw2023_problem;
mod solver_cycles;
mod solver_statespace;
mod state;

use std::path::PathBuf;
use std::str::FromStr;
use structopt::StructOpt;

#[derive(Debug)]
pub enum FileFormat {
    Raw2021Problem,
    Raw2023Problem,
    TrainsFormat,
}

impl FromStr for FileFormat {
    type Err = &'static str;
    fn from_str(day: &str) -> Result<Self, Self::Err> {
        match day {
            "raw2021" => Ok(FileFormat::Raw2021Problem),
            "raw2023" => Ok(FileFormat::Raw2023Problem),
            "trains" => Ok(FileFormat::TrainsFormat),
            _ => Err("Could not parse file format type."),
        }
    }
}

#[derive(Debug, StructOpt)]
#[structopt(name = "deadlockrail", about = "Railway deadlock checker.")]
struct Opt {
    /// Problem instance
    #[structopt(name = "FILE")]
    #[structopt(parse(from_os_str))]
    file: PathBuf,

    /// Write plan JSON if instance is not deadlocked.
    #[structopt(short)]
    #[structopt(parse(from_os_str))]
    planoutputfile: Option<PathBuf>,

    /// Activate debug mode
    #[structopt(short, long)]
    verbose: bool,

    /// File format to read. "raw" reads the Sasso benchmark instances.
    /// "trains" reads the two train scaling benchmark instances.
    #[structopt(long, default_value = "raw2021")]
    file_format: FileFormat,

    /// Choose between algorithms:
    /// 1 = algorithm simply adds states with consistency constraints and
    ///     solves until some upper bound of states is reached.
    /// 2 = extends 1 by adding a global progress constraints and solving
    ///     the system without the goal state assumption to check whether a deadlock
    ///     has been reached.
    /// 3 = extends 2 by adding local progress constraints which forces allocation
    ///     and freeing to happen as early as possible.
    /// 4 = alternative graph cycle checker
    #[structopt(long)]
    algorithm: Option<u8>,
}

fn main() {
    let _h1 = hprof::enter("init");

    let opt = Opt::from_args();
    let level = if opt.verbose {
        if cfg!(debug_assertions) {
            LevelFilter::Debug
        } else {
            LevelFilter::Info
        }
    } else {
        LevelFilter::Error
    };
    simple_logger::SimpleLogger::new()
        .with_level(level)
        .init()
        .unwrap();
    info!("{:#?}", opt);
    drop(_h1);

    let problem = {
        let json_contents = {
            let _h = hprof::enter("read file");
            trace!("Loading file {}", opt.file.to_str().unwrap());
            std::fs::read_to_string(&opt.file).unwrap()
        };

        let problem = {
            let _h = hprof::enter("parse");
            match opt.file_format {
                FileFormat::Raw2021Problem => {
                    let problem = {
                        let raw_problem: raw2021_problem::Problem =
                            serde_json::from_str(&json_contents).unwrap();
                        trace!(
                            "Converting problem with {} trains {} routes",
                            raw_problem.trains.len(),
                            raw_problem.routes.len()
                        );

                        problem::convert_raw2021(&raw_problem)
                    };
                    problem
                }
                FileFormat::Raw2023Problem => {
                    let problem = {
                        let h = hprof::enter("raw parse");

                        let raw_problem: raw2023_problem::Problem =
                            serde_json::from_str(&json_contents).unwrap();
                        trace!(
                            "Converting problem with {} trains {} routes",
                            raw_problem.trains.len(),
                            raw_problem.routes.len()
                        );

                        drop(h);
                        let _h = hprof::enter("convert");
                        // trace!("{:?}", raw_problem);

                        problem::convert_raw2023(&raw_problem)
                    };
                    problem
                }
                FileFormat::TrainsFormat => {
                    let problem: problem::Problem = serde_json::from_str(&json_contents).unwrap();
                    problem
                }
            }
        };
        problem
    };

    {
        let _h = hprof::enter("deadlockcheck");

        let result = match opt.algorithm {
            // None => {
            //     let (result2, tx) = mpsc::channel();
            //     let result1 = result2.clone();
            //     let rc1 = Arc::new(problem);
            //     let rc2 = rc1.clone();
            //     std::thread::spawn(move || {
            //         let _ = result1.send(solver_statespace::solve_1_using_num_states_bound(&*rc1));
            //     });
            //     std::thread::spawn(move || {
            //         let _ = result2.send(
            //             solver_statespace::solve_3_using_local_and_global_progress(&*rc2),
            //         );
            //     });

            //     tx.recv().unwrap()
            // }
            Some(1) => solver_statespace::solve_1_using_num_states_bound(&problem),
            Some(2) => solver_statespace::solve_2_using_global_progress(&problem),
            Some(3) => solver_statespace::solve_3_using_local_and_global_progress(&problem),
            Some(4) => solver_cycles::solve(&problem, idl::IdlSolver::new()),
            Some(5) => {
                let z3_ctx = z3::Context::new(&Default::default());
                let z3_solver = z3::Solver::new(&z3_ctx);
                let z3_object = (&z3_ctx, z3_solver);
                solver_cycles::solve(&problem, z3_object)
            }
            _ => panic!("Invalid algorithm {:?}", opt.algorithm),
        };

        match result {
            plan::DeadlockResult::Live(plan) => {
                info!("Plan found.");
                let (summary, commands) = plan::print_plan(&plan);
                debug!("Plan:\n{}", summary);
                debug!("Commands:\n{}", commands);
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

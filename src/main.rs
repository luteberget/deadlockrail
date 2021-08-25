use log::*;

mod plan;
mod problem;
mod raw_problem;
mod solver;
mod state;

use std::path::PathBuf;
use std::str::FromStr;
use structopt::StructOpt;

#[derive(Debug)]
pub enum FileFormat {
    RawProblem,
    TrainsFormat,
}

impl FromStr for FileFormat {
    type Err = &'static str;
    fn from_str(day: &str) -> Result<Self, Self::Err> {
        match day {
            "raw" => Ok(FileFormat::RawProblem),
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
    #[structopt(long, default_value = "raw")]
    file_format: FileFormat,

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
                FileFormat::RawProblem => {
                    let problem = {
                        let raw_problem: raw_problem::Problem =
                            serde_json::from_str(&json_contents).unwrap();
                        trace!(
                            "Converting problem with {} trains {} routes",
                            raw_problem.trains.len(),
                            raw_problem.routes.len()
                        );
                        problem::parse(&raw_problem)
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

        let result = match opt.algorithm.unwrap_or(3) {
            1 => solver::solve_1_using_num_states_bound(&problem),
            2 => solver::solve_2_using_global_progress(&problem),
            3 => solver::solve_3_using_local_and_global_progress(&problem),
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

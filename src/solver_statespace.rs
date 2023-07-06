use crate::plan::DeadlockResult;
use crate::state::{ub_steps, StateConstraintSettings};
use crate::{
    problem::*,
    state::{goal_condition, initial_state, mk_state, model_to_plan},
};
use log::*;
use satcoder::prelude::*;

pub fn solve_1_using_num_states_bound(problem: &Problem) -> DeadlockResult {
    // The first algorithm simply adds states with consistency constraints and
    // solves until some upper bound of states is reached.

    // This is an implementation of Algorithm 1 from the paper.

    solve(
        problem,
        false,
        StateConstraintSettings {
            global_progress_constraint: false,
            local_early_progress_constraint: false,
        },
    )
}

pub fn solve_2_using_global_progress(problem: &Problem) -> DeadlockResult {
    // The second algorithm adds a global progress constraints and solves
    // the system without the goal state assumption to check whether a deadlock
    // has been reached.

    // This is an implementation of Algorithm 2 from the paper.

    solve(
        problem,
        true,
        StateConstraintSettings {
            global_progress_constraint: true,
            local_early_progress_constraint: false,
        },
    )
}

pub fn solve_3_using_local_and_global_progress(problem: &Problem) -> DeadlockResult {
    // The third algorithm adds a local progress constraints which
    // 1. forces freeing of resources once sufficient resources ahead have been allocated, and
    // 2. forces allocation to happen as early as possible by allowing allocations to
    //    happen only in the step after they have been freed.

    // This is an implementation of Algorithm 3, the final one, from the paper.

    solve(
        problem,
        true,
        StateConstraintSettings {
            global_progress_constraint: true,
            local_early_progress_constraint: true,
        },
    )
}

pub fn solve(
    problem: &Problem,
    check_unconditional: bool,
    settings: StateConstraintSettings,
) -> DeadlockResult {
    let mut s = satcoder::solvers::cadical::Solver::new();
    // let mut s = satcoder::solvers::minisat::Solver::new();
    let mut states = vec![initial_state(&mut s, problem)];

    let ub = ub_steps(problem);

    let routenames = problem
        .trains
        .iter()
        .flat_map(|t| t.routes.keys())
        .collect::<std::collections::HashSet<_>>();

    warn!(
        "PROBLEM_STATS: {} trains {} routes {} routenames",
        problem.trains.len(),
        problem.trains.iter().map(|t| t.routes.len()).sum::<usize>(),
        routenames.len(),
    );

    loop {
        info!("Solver {:?}", s);
        info!(" with {} states", states.len());

        //
        // First solve unconditionally, to check that we haven't
        // deadlocked.
        //
        if check_unconditional {
            let _h = hprof::enter("unconditional solve");
            info!("Solving for unconditional progress");

            let uncond_result = SatSolver::solve(&mut s);
            if let SatResult::Sat(model) = uncond_result {
                info!("Unconditional solve succeeded.");
                let plan = model_to_plan(&states, problem, model.as_ref());
                let (summary, _commands) = crate::plan::print_plan(&plan);
                debug!("SUMMARY: \n{}\n", summary);
            } else {
                // TODO deadlock analysis
                info!("DEADLOCKED: situation locked in {} steps", states.len());
                return DeadlockResult::Deadlocked(());
            }
        }

        //
        // Now let's see if we are Live:
        //
        if let Some(goal) = goal_condition(states.last().unwrap()) {
            info!("Solving for goal state");
            let _h = hprof::enter("goal solve");
            let result = SatSolverWithCore::solve_with_assumptions(&mut s, goal);

            if let SatResultWithCore::Sat(model) = result {
                info!("LIVE: situation resolved in {} steps", states.len());
                let plan = model_to_plan(&states, problem, model.as_ref());
                drop(model);
                return DeadlockResult::Live(plan);
            } else {
                info!("Goal condition not satisfised.");
            }
        } else {
            trace!("Goal conditions was statically unsat.");
        }

        //
        // No conclusion, we need to add another state.
        //
        if states.len() - 1 < ub {
            let _p = hprof::enter("add step");
            states.push(mk_state(
                &mut s,
                states.last().unwrap(),
                problem,
                states.len(),
                settings,
            ));
        } else {
            info!(
                "DEADLOCKED: situation locked after reaching UB {} on steps at {}",
                ub,
                states.len()
            );
            return DeadlockResult::Deadlocked(());
        }
    }
}

use std::{collections::HashMap, path::Path};
use serde_json::json;


#[derive(Debug)]
pub enum DeadlockResult {
    Live(Plan),
    Deadlocked(()),
}

#[derive(Debug)]
pub struct Plan {
    pub steps: Vec<Vec<(String, Option<TrainId>)>>,
}

pub type RouteId = String;
pub type TrainId = usize;


pub fn write_plan_json(filename: &Path, plan: Plan) -> std::io::Result<()> {
    std::fs::write(filename, serde_json::to_string_pretty(&plan_json(plan))?)?;
    Ok(())
}

fn plan_json(plan: Plan) -> serde_json::Value {
    let steps = plan
        .steps
        .iter()
        .map(|s| s.iter().cloned().collect::<HashMap<String, _>>())
        .collect::<Vec<_>>();
    json!({ "steps": steps })
}

pub fn print_plan(plan: &Plan) -> (String, String) {
    let mut summary = String::new();
    let mut commands = Vec::new();
    for (step_n, step) in plan.steps.iter().enumerate() {
        while commands.len() < step_n + 1 {
            commands.push(Vec::new());
        }
        for (r, t) in step.iter() {
            if let Some(t) = t {
                let already_allocated = step_n > 0
                    && plan.steps[step_n - 1]
                        .iter()
                        .find(|(prev_r, _o)| r == prev_r)
                        .unwrap()
                        .1
                        == Some(*t);
                if !already_allocated {
                    commands.last_mut().unwrap().push(format!("t{} r{}", t, r))
                }
            }
        }

        summary.push_str(&format!("Step {}: ", step_n));
        summary.push_str(
            &step
                .iter()
                .map(|(r, t)| {
                    format!(
                        "{:>8} {:<3}",
                        format!("r{}", r),
                        if let Some(t) = t {
                            if step_n > 0
                                && plan.steps[step_n - 1]
                                    .iter()
                                    .find(|(prev_r, _o)| r == prev_r)
                                    .unwrap()
                                    .1
                                    == Some(*t)
                            {
                                format!(".{}", t)
                            } else {
                                format!("*{}", t)
                            }
                        } else {
                            "___".to_string()
                        }
                    )
                })
                .collect::<Vec<_>>()
                .join(" "),
        );
        summary.push('\n');
    }
    let commands = commands
        .into_iter()
        .map(|c| c.join("\n"))
        .collect::<Vec<_>>()
        .join("\n\n");
    (summary, commands)
}

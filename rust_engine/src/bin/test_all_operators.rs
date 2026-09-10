#[path = "../core/mod.rs"]
mod core;

use std::collections::HashMap;

fn main() {
    println!("=== TESTING ALL 438 OPERATORS IN RUST ENGINE ===");
    let loader = core::data_loader::DataLoader::new("../data");
    let op_names = loader.get_all_operator_names();
    let count = op_names.len();
    println!("Loaded operators count: {}", count);

    let avg_enemy = core::enemy::calculate_average_enemy("../data");
    println!("Average Enemy: DEF={:.1}, RES={:.1}, HP={:.1}, ATK={:.1}, Interval={:.2}s",
        avg_enemy.def, avg_enemy.res, avg_enemy.hp, avg_enemy.atk, avg_enemy.attack_interval);

    let mut target_stats = HashMap::new();
    target_stats.insert("def".to_string(), avg_enemy.def);
    target_stats.insert("res".to_string(), avg_enemy.res);
    target_stats.insert("atk".to_string(), avg_enemy.atk);
    target_stats.insert("attack_interval".to_string(), avg_enemy.attack_interval);
    target_stats.insert("weight".to_string(), avg_enemy.weight);
    target_stats.insert("hp".to_string(), avg_enemy.hp);

    let mut success = 0;
    let mut failures = Vec::new();

    for name in &op_names {
        if let Some(op) = loader.get_operator(name) {
            let mut sim = core::simulation::SimulationEnvironment::new(op.clone(), None, Some(target_stats.clone()));
            let (dmg, heal, dp, _, _, _) = sim.run_5_minute_sim();
            let total_dmg = dmg.last().map(|&(_, v)| v).unwrap_or(0.0);
            let total_heal = heal.last().map(|&(_, v)| v).unwrap_or(0.0);
            let total_dp = dp.last().map(|&(_, v)| v).unwrap_or(0.0);

            if total_dmg.is_nan() || total_heal.is_nan() || total_dp.is_nan() {
                failures.push((name.clone(), "NaN output".to_string()));
            } else {
                success += 1;
            }
        } else {
            failures.push((name.clone(), "Failed to get operator".to_string()));
        }
    }

    println!("\nSimulation test results: {}/{} succeeded.", success, count);
    if !failures.is_empty() {
        println!("Failures: {:?}", failures);
    } else {
        println!("ALL {} OPERATORS SIMULATED CLEANLY WITH ZERO ERRORS/NAN!", success);
    }
}

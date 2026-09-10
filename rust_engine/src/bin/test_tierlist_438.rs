#![allow(unused_variables, unused_mut)]
#[path = "../core/mod.rs"]
mod core;

use std::collections::HashMap;

fn main() {
    println!("=== TESTING FULL TIER LIST EVALUATION FOR 438 OPERATORS ===");
    let loader = core::data_loader::DataLoader::new("../data");
    let op_names = loader.get_all_operator_names();
    println!("Total operators to evaluate: {}", op_names.len());

    let avg_enemy = core::enemy::calculate_average_enemy("../data");

    let mut target_stats = HashMap::new();
    target_stats.insert("def".to_string(), avg_enemy.def);
    target_stats.insert("res".to_string(), avg_enemy.res);
    target_stats.insert("atk".to_string(), avg_enemy.atk);
    target_stats.insert("attack_interval".to_string(), avg_enemy.attack_interval);
    target_stats.insert("weight".to_string(), avg_enemy.weight);
    target_stats.insert("hp".to_string(), avg_enemy.hp);

    let mut all_results = Vec::new();

    for name in &op_names {
        if let Some(mut op) = loader.get_operator(name) {
            // Find best skill
            let mut best_score = 0.0;
            let mut best_dps = 0.0;
            let mut best_sk = "No Skill".to_string();

            let skills_to_eval: Vec<Option<usize>> = if op.skills.is_empty() {
                vec![None]
            } else {
                (0..op.skills.len()).map(Some).collect()
            };

            for sk_opt in skills_to_eval {
                let mut op_clone = op.clone();
                let sk_name = if let Some(idx) = sk_opt {
                    op_clone.equipped_skill = Some(op_clone.skills[idx].clone());
                    op_clone.skills[idx].name.clone()
                } else {
                    op_clone.equipped_skill = None;
                    "No Skill".to_string()
                };

                let mut sim = core::simulation::SimulationEnvironment::new(op_clone.clone(), None, Some(target_stats.clone()));
                let (dmg, heal, _, _, _, _) = sim.run_5_minute_sim();
                let total_dmg = dmg.last().map(|&(_, v)| v).unwrap_or(0.0);
                let total_heal = heal.last().map(|&(_, v)| v).unwrap_or(0.0);
                let dps = total_dmg / 300.0;
                let hps = total_heal / 300.0;
                let ehp = op_clone.calculate_ehp_phys_against(avg_enemy.atk);

                let score = dps + hps * 1.5 + (ehp / 1000.0);
                if score > best_score {
                    best_score = score;
                    best_dps = dps;
                    best_sk = sk_name;
                }
            }

            all_results.push((name.clone(), op.rarity, best_sk, best_dps, best_score));
        }
    }

    all_results.sort_by(|a, b| b.4.partial_cmp(&a.4).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n--- TOP 20 OPERATORS (out of {}) ---", all_results.len());
    println!("{:<4} {:<28} {:<8} {:<10} {:<10}", "Rank", "Operator", "Rarity", "DPS", "Score");
    println!("{}", "-".repeat(65));
    for (i, (name, rarity, sk, dps, score)) in all_results.iter().take(20).enumerate() {
        println!("{:<4} {:<28} {:<8} {:<10.1} {:<10.1}", i + 1, name, format!("{}*", rarity), dps, score);
    }
}

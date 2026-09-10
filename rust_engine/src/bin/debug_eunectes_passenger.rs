#![allow(unused_variables)]
#[path = "../core/mod.rs"]
mod core;

use std::collections::HashMap;

fn main() {
    let loader = core::data_loader::DataLoader::new("../data");
    let avg_enemy = core::enemy::calculate_average_enemy("../data");

    let mut target_stats = HashMap::new();
    target_stats.insert("def".to_string(), avg_enemy.def);
    target_stats.insert("res".to_string(), avg_enemy.res);
    target_stats.insert("atk".to_string(), avg_enemy.atk);
    target_stats.insert("attack_interval".to_string(), avg_enemy.attack_interval);
    target_stats.insert("weight".to_string(), avg_enemy.weight);
    target_stats.insert("hp".to_string(), avg_enemy.hp);

    for name in &["Eunectes", "Passenger"] {
        println!("\n================ {} ================", name);
        if let Some(op) = loader.get_operator(name) {
            println!("Class ID: {:?}, Branch ID: {:?}", op.class_id_num(), op.branch_id_num());
            println!("Base interval: {:.2}s, ATK: {:.1}", op.final_interval(), op.final_atk());

            for (sk_idx, sk) in op.skills.iter().enumerate() {
                let mut op_clone = op.clone();
                op_clone.equipped_skill = Some(sk.clone());

                let mut sim = core::simulation::SimulationEnvironment::new(op_clone.clone(), None, Some(target_stats.clone()));
                let (dmg, heal, dp, _, _, split) = sim.run_5_minute_sim();

                let total_dmg = dmg.last().map(|&(_, v)| v).unwrap_or(0.0);
                let dps = total_dmg / 300.0;

                // Let's also inspect state_rates
                op_clone.is_skill_active = false;
                let b_rates = core::simulation::SimulationEnvironment::new(op_clone.clone(), None, Some(target_stats.clone())).state_rates();
                op_clone.is_skill_active = true;
                let s_rates = core::simulation::SimulationEnvironment::new(op_clone.clone(), None, Some(target_stats.clone())).state_rates();

                println!("  Skill {}: {} (sp_cost={}, init={}, dur={}, sp_type={})",
                    sk_idx + 1, sk.name, sk.sp_cost, sk.initial_sp, sk.duration, sk.sp_type);
                println!("    Base rates: atk={:.1}, aps={:.2}, phys/shot={:.1}, arts/shot={:.1}",
                    b_rates.atk, b_rates.attacks_per_sec, b_rates.phys_per_shot, b_rates.arts_per_shot);
                println!("    Skill rates: atk={:.1}, aps={:.2}, phys/shot={:.1}, arts/shot={:.1}, end_burst={:.1}",
                    s_rates.atk, s_rates.attacks_per_sec, s_rates.phys_per_shot, s_rates.arts_per_shot, s_rates.end_burst_raw);
                println!("    Total 5-min DMG: {:.0} (DPS: {:.1})", total_dmg, dps);
                println!("    Split: {:?}", split);
            }
        }
    }
}

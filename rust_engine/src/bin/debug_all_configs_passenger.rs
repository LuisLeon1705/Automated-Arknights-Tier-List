#![allow(unused_variables)]
#[path = "../core/mod.rs"]
mod core;

use std::collections::HashMap;

fn main() {
    let loader = core::data_loader::DataLoader::new("../data");
    let avg_enemy = core::enemy::calculate_average_enemy("../data");

    for name in &["Passenger", "Eunectes"] {
        println!("\n================ {} ================", name);
        if let Some(op) = loader.get_operator(name) {
            let modules = if op.modules.is_empty() { vec![None] } else {
                let mut v = vec![None];
                for i in 0..op.modules.len() { v.push(Some(i)); }
                v
            };

            let skills = if op.skills.is_empty() { vec![None] } else {
                let mut v = vec![None];
                for i in 0..op.skills.len() { v.push(Some(i)); }
                v
            };

            for mod_opt in &modules {
                for sk_opt in &skills {
                    let mut op_clone = op.clone();
                    if let Some(m_idx) = mod_opt {
                        op_clone.active_module = Some(op_clone.modules[*m_idx].clone());
                    }
                    if let Some(s_idx) = sk_opt {
                        op_clone.equipped_skill = Some(op_clone.skills[*s_idx].clone());
                    }

                    let sk_name = op_clone.equipped_skill.as_ref().map(|s| s.name.clone()).unwrap_or("RAW".to_string());
                    let mod_name = op_clone.active_module.as_ref().map(|m| m.name.clone()).unwrap_or("No Module".to_string());

                    let mut target_stats = HashMap::new();
                    target_stats.insert("def".to_string(), avg_enemy.def);
                    target_stats.insert("res".to_string(), avg_enemy.res);
                    target_stats.insert("weight".to_string(), avg_enemy.weight);
                    target_stats.insert("atk".to_string(), avg_enemy.atk);
                    target_stats.insert("attack_interval".to_string(), avg_enemy.attack_interval);
                    target_stats.insert("is_boss".to_string(), 0.0);

                    let mut sim = core::simulation::SimulationEnvironment::new(op_clone.clone(), None, Some(target_stats));
                    let (dmg, heal, dp, _, _, dmg_split) = sim.run_5_minute_sim();

                    let arts = dmg_split.get("arts").copied().unwrap_or(0.0);
                    let phys = dmg_split.get("physical").copied().unwrap_or(0.0);
                    let pot_aoe = dmg_split.get("potential_aoe").copied().unwrap_or(0.0);
                    let burst = dmg_split.get("max_damage_per_cast").copied().unwrap_or(0.0);

                    // Check if ratio_res or ratio_def is huge!
                    let (flat_def, ratio_def) = op_clone.target_def_debuffs();
                    let (flat_res, ratio_res) = op_clone.target_res_debuffs();

                    let score_arts_dmg = arts * (1.0 + ratio_res.abs()) * (1.0 + op_clone.arts_fragile()) * (1.0 + op_clone.fragile());

                    if score_arts_dmg > 500_000.0 || pot_aoe > 1_000_000.0 {
                        println!("  >>> INFLATED DETECTED: Skill={}, Mod={}", sk_name, mod_name);
                        println!("      arts={}, ratio_res={}, arts_fragile={}, fragile={}", arts, ratio_res, op_clone.arts_fragile(), op_clone.fragile());
                        println!("      score_arts_dmg={}, pot_aoe={}, burst={}", score_arts_dmg, pot_aoe, burst);
                        println!("      dmg_split: {:?}", dmg_split);
                    }
                }
            }
        }
    }
}

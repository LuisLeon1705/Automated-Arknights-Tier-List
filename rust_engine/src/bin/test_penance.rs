#![allow(unused_variables)]
#[path = "../core/mod.rs"]
mod core;

use std::collections::HashMap;

fn main() {
    println!("=== TESTING PENANCE IN RUST ENGINE ===");
    let loader = core::data_loader::DataLoader::new("../data");
    let avg_enemy = core::enemy::calculate_average_enemy("../data");

    println!("Average Enemy Stats: DEF={:.1}, RES={:.1}, HP={:.1}, ATK={:.1}, Interval={:.2}s",
        avg_enemy.def, avg_enemy.res, avg_enemy.hp, avg_enemy.atk, avg_enemy.attack_interval);

    let penance = loader.get_operator("Penance").expect("Failed to find Penance in DataLoader!");

    println!("Penance Loaded: Name={}, HP={}, ATK={}, DEF={}, RES={}, Block={}",
        penance.name,
        penance.final_hp(),
        penance.final_atk(),
        penance.final_def(),
        penance.calculate_stat("res"),
        penance.target_limit());

    println!("Skills count: {}, Modules count: {}", penance.skills.len(), penance.modules.len());

    let mut target_stats = HashMap::new();
    target_stats.insert("def".to_string(), avg_enemy.def);
    target_stats.insert("res".to_string(), avg_enemy.res);
    target_stats.insert("atk".to_string(), avg_enemy.atk);
    target_stats.insert("attack_interval".to_string(), avg_enemy.attack_interval);
    target_stats.insert("weight".to_string(), avg_enemy.weight);
    target_stats.insert("hp".to_string(), avg_enemy.hp);

    let skills_to_test = vec![None, Some(0), Some(1), Some(2)];
    let modules_to_test = vec![None, if !penance.modules.is_empty() { Some(0) } else { None }];

    for mod_opt in &modules_to_test {
        let mod_label = match mod_opt {
            Some(idx) => format!("Module {}", idx + 1),
            None => "No Module".to_string(),
        };

        println!("\n--- Testing with {} ---", mod_label);

        for sk_opt in &skills_to_test {
            let mut op = penance.clone();
            if let Some(m_idx) = mod_opt {
                op.active_module = Some(op.modules[*m_idx].clone());
            }
            let sk_name = match sk_opt {
                Some(idx) => {
                    op.equipped_skill = Some(op.skills[*idx].clone());
                    op.skills[*idx].name.clone()
                },
                None => {
                    op.equipped_skill = None;
                    "No Skill (Raw)".to_string()
                }
            };

            let init_bar = op.initial_barrier();
            let skill_bar = op.skill_barrier();
            let cap = op.max_barrier_cap();
            let thorns = op.thorns_reflect_ratio();
            let ehp_phys = op.calculate_ehp_phys_against(avg_enemy.atk);
            let ehp_arts = op.calculate_ehp_arts_against(avg_enemy.atk);

            let mut sim = core::simulation::SimulationEnvironment::new(op.clone(), None, Some(target_stats.clone()));
            let (dmg, heal, dp, _, _, dmg_split) = sim.run_5_minute_sim();

            let total_dmg = dmg.last().map(|&(_, v)| v).unwrap_or(0.0);
            let phys_dmg = dmg_split.get("physical").copied().unwrap_or(0.0);
            let arts_dmg = dmg_split.get("arts").copied().unwrap_or(0.0);
            let reflect_arts = dmg_split.get("reflected_arts").copied().unwrap_or(0.0);

            let mut boss_sim = core::simulation::SimulationEnvironment::new(op.clone(), None, Some(target_stats.clone()));
            // Standard boss benchmark: 100k HP, 1000 DEF, 50 RES
            let (_boss_ttc_raw, boss_ttc, boss_leak) = boss_sim.run_boss_sim(100_000.0, 1000.0, 50.0);

            println!("\n  Skill: {}", sk_name);
            println!("    Initial Barrier: {:.1} HP, Skill Barrier: {:.1} HP (Max Cap: {:.1} HP)", init_bar, skill_bar, cap);
            println!("    Thorns Ratio: {:.2}%, EHP Phys: {:.0}, EHP Arts: {:.0}", thorns * 100.0, ehp_phys, ehp_arts);
            println!("    5-min Total DMG: {:.0} (DPS: {:.1})", total_dmg, total_dmg / 300.0);
            println!("    DMG Split: Phys={:.0}, Arts={:.0} (Reflected Thorns Arts={:.0})", phys_dmg, arts_dmg, reflect_arts);
            println!("    Boss (100k HP, 1000 DEF, 50 RES) Time-to-Kill: {:.1}s (Leak: {:.1}%)", boss_ttc, boss_leak * 100.0);
        }
    }

    println!("\n=== PENANCE TEST COMPLETED SUCCESSFULLY ===");
}

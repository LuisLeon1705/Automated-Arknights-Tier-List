#[path = "../core/mod.rs"]
mod core;

fn main() {
    let loader = core::data_loader::DataLoader::new("../data");
    test_operator(&loader, "Ines");
}

fn test_operator(loader: &core::data_loader::DataLoader, name: &str) {
    println!("\n=======================================================");
    println!("TESTING OPERATOR: {}", name);
    println!("=======================================================");
    let op_opt = loader.get_operator(name);
    if op_opt.is_none() {
        println!("Could not load operator {}", name);
        return;
    }
    let base_op = op_opt.unwrap();
    println!("Rarity: {} | Class: {} | Subclass: {} | Duelist: {}", 
        base_op.rarity, base_op.profession, base_op.subclass_name, base_op.is_duelist());
    println!("DEF Ignore Ratio: {:.2}% | RES Ignore Ratio: {:.2}%", 
        base_op.def_ignore_ratio() * 100.0, base_op.res_ignore_ratio() * 100.0);

    let avg_enemy = core::enemy::calculate_average_enemy("../data");
    let boss_enemy = core::enemy::get_enemy_by_category("../data", "boss");
    let mut target_stats = std::collections::HashMap::new();
    target_stats.insert("def".to_string(), avg_enemy.def);
    target_stats.insert("res".to_string(), avg_enemy.res);
    target_stats.insert("weight".to_string(), avg_enemy.weight);
    target_stats.insert("atk".to_string(), avg_enemy.atk);
    target_stats.insert("attack_interval".to_string(), avg_enemy.attack_interval);
    target_stats.insert("is_boss".to_string(), 0.0);

    for (s_idx, s) in base_op.skills.iter().enumerate() {
        let mut op = base_op.clone();
        op.equipped_skill = Some(s.clone());
        op.change_state(Some(s_idx.to_string()));

        let mut sim = core::simulation::SimulationEnvironment::new(op.clone(), None, Some(target_stats.clone()));
        let (_cycle, _burst_end) = sim.cycle_at(avg_enemy.def, avg_enemy.res);
        let (_dmg_t, _heal_t, _dp_t, _dmg_e, _heal_e, dmg_split) = sim.run_5_minute_sim();
        let wave_ttc = sim.run_wave_sim(avg_enemy.hp, avg_enemy.def, avg_enemy.res);
        let boss_ttc = sim.run_boss_sim(80000.0, 1200.0, 50.0);

        let phys = dmg_split.get("physical").copied().unwrap_or(0.0);
        let arts = dmg_split.get("arts").copied().unwrap_or(0.0);
        let true_d = dmg_split.get("true").copied().unwrap_or(0.0);
        let ele = dmg_split.get("elemental").copied().unwrap_or(0.0);
        let total_dmg = phys + arts + true_d + ele;

        let arts_surv = op.calculate_ehp_arts_against(avg_enemy.atk);
        let phys_surv = op.calculate_ehp_phys_against(avg_enemy.atk);
        let base_surv = (arts_surv + phys_surv) / 2.0;
        let survivability = base_surv * (1.0 + op.calculate_stat("healing_received_bonus"));
        let hits_to_kill = op.calculate_hits_to_kill(avg_enemy.atk, avg_enemy.attack_interval, 0.20);
        let surv_score = (survivability / 300.0) * 0.35 + (hits_to_kill.min(50.0) * 12.0);

        let boss_arts_surv = op.calculate_ehp_arts_against(boss_enemy.atk);
        let boss_phys_surv = op.calculate_ehp_phys_against(boss_enemy.atk);
        let boss_surv = ((boss_arts_surv + boss_phys_surv) / 2.0) * (1.0 + op.calculate_stat("healing_received_bonus"));
        let boss_htk = op.calculate_hits_to_kill(boss_enemy.atk, boss_enemy.attack_interval, 0.20);

        let field_block = op.total_field_block();
        let mut block_score = 0.0;
        if op.is_defender() {
            if field_block < 2.0 {
                block_score = -(3.0 - field_block) * 300.0;
            } else if field_block > 3.0 {
                block_score = (field_block - 3.0) * 150.0;
            }
        } else if field_block >= 3.0 {
            block_score = (field_block - 2.0) * 120.0;
        }
        let total_score = (total_dmg / 300.0) + surv_score + block_score;

        println!("  Skill [{}]: {}", s_idx + 1, s.name);
        println!("    Dur: {:.1}s | SPCost: {} | SPType: {}", s.duration, s.sp_cost, s.sp_type);
        println!("    5-min Total Dmg: {:.0} (DPS: {:.1}) | Phys: {:.0} | Arts: {:.0} | True: {:.0}",
            total_dmg, total_dmg / 300.0, phys, arts, true_d);
        println!("    Surv: {:.0} | HTK: {:.1} | SurvScore: {:.1} | FieldBlock: {:.1} | BlockAdj: {:.1}",
            survivability, hits_to_kill, surv_score, field_block, block_score);
        println!("    [BOSS] Surv: {:.0} | HTK: {:.1} | BossPhysEHP: {:.0} | BossArtsEHP: {:.0}",
            boss_surv, boss_htk, boss_phys_surv, boss_arts_surv);
        println!("    TotalScore: {:.1} | Wave TTC: {:.1}s | Boss TTC: {:.1}s", total_score, wave_ttc, boss_ttc);
    }
}

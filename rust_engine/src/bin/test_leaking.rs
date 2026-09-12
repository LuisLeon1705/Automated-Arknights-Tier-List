#[path = "../core/mod.rs"]
mod core;

use std::collections::HashMap;

fn boss_target_stats(boss: &core::enemy::AverageEnemy) -> HashMap<String, f64> {
    let mut ts = HashMap::new();
    ts.insert("def".to_string(), boss.def);
    ts.insert("res".to_string(), boss.res);
    ts.insert("weight".to_string(), boss.weight);
    ts.insert("atk".to_string(), boss.atk);
    ts.insert("attack_interval".to_string(), boss.attack_interval);
    ts.insert("is_boss".to_string(), 1.0);
    ts.insert("hp".to_string(), boss.hp);
    ts
}

fn eval(loader: &core::data_loader::DataLoader, avg: &core::enemy::AverageEnemy, boss: &core::enemy::AverageEnemy, name: &str) {
    let op_opt = loader.get_operator(name);
    if op_opt.is_none() {
        println!("  [SKIP] {} not found", name);
        return;
    }
    let base_op = op_opt.unwrap();
    for (i, s) in base_op.skills.iter().enumerate() {
        let mut op = base_op.clone();
        op.equipped_skill = Some(s.clone());
        op.change_state(Some(i.to_string()));

        let mut wave_sim = core::simulation::SimulationEnvironment::new(op.clone(), None, Some(boss_target_stats(avg)));
        let (_wave_ttc_raw, wave_ttc, wave_leaks) = wave_sim.run_wave_sim(avg.hp, avg.def, avg.res);

        let mut boss_sim = core::simulation::SimulationEnvironment::new(op.clone(), None, Some(boss_target_stats(boss)));
        let (_boss_ttc_raw, boss_ttc, boss_leak) = boss_sim.run_boss_sim(80000.0, 1200.0, 50.0);

        println!("  S{} [{}]: Wave {:.1}s (leaks {:.1}/100) | Boss {:.1}s (leak {:.1}%)",
            i + 1, s.name, wave_ttc, wave_leaks, boss_ttc, boss_leak * 100.0);
    }
}

fn main() {
    let loader = core::data_loader::DataLoader::new("../data");
    let avg = core::enemy::calculate_average_enemy("../data");
    let boss = core::enemy::calculate_boss_enemy("../data");
    println!("avg hp={} def={} res={} weight={}", avg.hp, avg.def, avg.res, avg.weight);

    println!("\n=== [Mon3tr] Talent & S2 scaling ===");
    let mon3tr = loader.get_operator("Mon3tr").unwrap();
    let t2_name = mon3tr.talents.get(1).map(|t| t.name.clone()).unwrap_or_default();
    let s2 = &mon3tr.skills[1];
    let tt = s2.buffs.iter().find(|b| b.stat == "talent_multiplier").and_then(|b| b.target_talent.clone());
    println!("  S2 talent_multiplier target_talent: {:?} (talent 2 name: {})", tt, t2_name);
    assert_eq!(tt, Some(t2_name.clone()), "S2 talent_scale must target the 2nd talent (战术协同)");

    let mut op_s2 = mon3tr.clone();
    op_s2.equipped_skill = Some(s2.clone());
    op_s2.change_state(Some("1".to_string()));
    let aspd_s2 = op_s2.calculate_stat("aspd");
    println!("  Mon3tr S2-active ASPD: {:.1} (expected ~61.6 = 22 * 2.8)", aspd_s2);
    assert!((aspd_s2 - 61.6).abs() < 1.0, "S2 must scale Talent 2 by 2.8x: got {}", aspd_s2);
    let int_s2 = op_s2.final_interval();
    println!("  Mon3tr S2-active interval: {:.3} (expected ~1.764 = 2.85 / 1.616)", int_s2);
    assert!((int_s2 - 1.7637).abs() < 0.05, "interval must reflect scaled ASPD: got {}", int_s2);

    println!("\n=== [Leaking] Wave & Boss categories ===");
    for name in &["Myrtle", "Surtr", "Ulpianus", "Crownslayer", "Bagpipe", "Mountain", "Exusiai"] {
        println!("{}", name);
        eval(&loader, &avg, &boss, name);
    }

    let mut myrtle = loader.get_operator("Myrtle").unwrap();
    let mi = myrtle.skills.iter().position(|s| s.name.contains("支援")) .or(Some(0)).unwrap();
    myrtle.equipped_skill = Some(myrtle.skills[mi].clone());
    myrtle.change_state(Some(mi.to_string()));
    let mut ms = core::simulation::SimulationEnvironment::new(myrtle.clone(), None, Some(boss_target_stats(&boss)));
    let (_myrtle_boss_ttc_raw, myrtle_boss_ttc, myrtle_boss_leak) = ms.run_boss_sim(80000.0, 1200.0, 50.0);
    println!("\nMyrtle boss: {:.1}s leak {:.1}% (must be ~100%: ranged, cannot hold or kill)", myrtle_boss_ttc, myrtle_boss_leak * 100.0);
    assert!(myrtle_boss_leak >= 0.9, "Myrtle must leak the boss ({}%)", myrtle_boss_leak * 100.0);

    println!("\n=== ALL LEAKING + MON3TR CHECKS PASSED ===");
}

#[path = "../core/mod.rs"]
mod core;

use std::collections::HashMap;

fn main() {
    let loader = core::data_loader::DataLoader::new("../data");

    println!("=== [1] prob disambiguation ===");
    let mountain = loader.get_operator("Mountain").unwrap();
    let t1 = &mountain.talents[0];
    let t2 = &mountain.talents[1];
    let t1_stats: Vec<&str> = t1.buffs.iter().map(|b| b.stat.as_str()).collect();
    let t2_stats: Vec<&str> = t2.buffs.iter().map(|b| b.stat.as_str()).collect();
    println!("  Mountain T1 ({}): {:?}", t1.name, t1_stats);
    println!("  Mountain T2 ({}): {:?}", t2.name, t2_stats);
    assert!(t1_stats.contains(&"prob"), "crit talent must keep 'prob' (trigger chance)");
    assert!(t2_stats.contains(&"arts_dodge"), "dodge talent must map to 'arts_dodge'");

    println!("\n=== [2] Mountain crit expected value (no longer 100%) ===");
    let mut m = mountain.clone();
    m.equipped_skill = Some(m.skills[1].clone());
    m.change_state(Some("1".to_string()));
    let mult_raw = m.damage_multiplier();
    println!("  S2 no module: dmg_mult = {:.4} (raw talent 1.65@20% -> ~1.13)", mult_raw);
    assert!(mult_raw < 1.25, "20% crit must be expected-valued, got {}", mult_raw);

    let fgt_y = m.modules.iter().find(|mm| mm.name.contains("FGT-Y")).cloned();
    if let Some(mod_y) = fgt_y {
        m.active_module = Some(mod_y);
        let mult_mod = m.damage_multiplier();
        println!("  S2 + FGT-Y: dmg_mult = {:.4} (upgraded 1.75@25% -> ~1.19, NOT 1.65*1.75=2.89)", mult_mod);
        assert!(mult_mod < 1.35, "module must upgrade (not stack) talent group, got {}", mult_mod);
    }

    let mut m_aoe = mountain.clone();
    m_aoe.equipped_skill = Some(m_aoe.skills[1].clone());
    m_aoe.change_state(Some("1".to_string()));
    let tl = m_aoe.target_limit();
    println!("  S2 target_limit = {:.1} (block AoE => block count, not uncapped 5)", tl);
    assert!(tl < 5.0, "blocking-all-enemies skill must be capped by block, got {}", tl);

    println!("\n=== [3] Proc burst passives (Iana S1 etc.) ===");
    let avg = core::enemy::calculate_average_enemy("../data");
    let mut ts: HashMap<String, f64> = HashMap::new();
    ts.insert("def".to_string(), avg.def);
    ts.insert("res".to_string(), avg.res);
    ts.insert("weight".to_string(), avg.weight);
    ts.insert("atk".to_string(), avg.atk);
    ts.insert("attack_interval".to_string(), avg.attack_interval);
    ts.insert("is_boss".to_string(), 0.0);

    for name in &["Iana", "Projekt Red", "Hoshiguma", "Phantom"] {
        let op = loader.get_operator(name).unwrap();
        let mut best_before = 0.0f64;
        let mut best_after = 0.0f64;
        for (i, s) in op.skills.iter().enumerate() {
            let mut o = op.clone();
            o.equipped_skill = Some(s.clone());
            o.change_state(Some(i.to_string()));
            let is_proc = (o.sub_profession_id == "dollkeeper" || s.description.contains("立即对") || s.description.contains("受到攻击"))
                && s.is_passive();
            let mut sim = core::simulation::SimulationEnvironment::new(o.clone(), None, Some(ts.clone()));
            let (dmg, _, _, _, _, _) = sim.run_5_minute_sim();
            let dps = dmg.last().map(|&(_, v)| v).unwrap_or(0.0) / 300.0;
            println!("  {} S{} [{}] proc={} DPS={:.0}", name, i + 1, s.name, is_proc, dps);
            if dps > best_after { best_after = dps; }
            // pre-fix reference: full scale applied permanently would inflate the proc skills
            if is_proc { best_before = dps; }
        }
        let _ = best_before;
    }

    let iana = loader.get_operator("Iana").unwrap();
    let mut io_ = iana.clone();
    io_.equipped_skill = Some(io_.skills[0].clone());
    io_.change_state(Some("0".to_string()));
    let dmg_mult_full = io_.damage_multiplier();
    println!("  Iana S1 raw damage_multiplier = {:.2} (4x scale); sim should apply duty-cycle damping so effective DPS << atk*4/interval", dmg_mult_full);
    assert!(dmg_mult_full > 3.5);

    let mut sim = core::simulation::SimulationEnvironment::new(io_.clone(), None, Some(ts.clone()));
    let (dmg, _, _, _, _, _) = sim.run_5_minute_sim();
    let iana_dps = dmg.last().map(|&(_, v)| v).unwrap_or(0.0) / 300.0;
    let undamped = io_.final_atk() * 4.0 / 1.2;
    println!("  Iana S1 300s DPS = {:.0} (undamped constant-burst would be {:.0})", iana_dps, undamped);
    assert!(iana_dps < undamped * 0.85, "Iana S1 must not run constantly on every attack: {}", iana_dps);

    println!("\n=== [4] Damage-capable medics actually deal damage ===");
    let med_5min = |name: &str, skill: usize| -> (f64, f64, f64) {
        let op = loader.get_operator(name).unwrap();
        let mut o = op.clone();
        o.equipped_skill = Some(o.skills[skill].clone());
        o.change_state(Some(skill.to_string()));
        let mut s = core::simulation::SimulationEnvironment::new(o.clone(), None, Some(ts.clone()));
        let (dmg, _, _, _, _, split) = s.run_5_minute_sim();
        (
            dmg.last().map(|x| x.1).unwrap_or(0.0) / 300.0,
            split.get("true").copied().unwrap_or(0.0) / 300.0,
            split.get("arts").copied().unwrap_or(0.0) / 300.0,
        )
    };
    let kal_s3 = med_5min("Kal'tsit", 2);
    println!("  Kal'tsit S3 (Mon3tr command): DPS={:.0} true={:.0}", kal_s3.0, kal_s3.1);
    assert!(kal_s3.1 > 100.0, "Kal'tsit S3 must deal true damage via Mon3tr: {}", kal_s3.1);
    let kal_s2 = med_5min("Kal'tsit", 1);
    assert!(kal_s2.0 > 100.0, "Kal'tsit S2 Mon3tr strikes must deal physical damage: {}", kal_s2.0);

    // Wandering medics are pure sustain: they must deal NO damage (Hvít Aska, branch 404)
    let hvit_s1 = med_5min("Eyjafjalla the Hvít Aska", 0);
    let hvit_s3 = med_5min("Eyjafjalla the Hvít Aska", 2);
    println!("  Hvít Aska S1: DPS={:.1} | S3: DPS={:.1} (must be 0: wander medics do not attack)", hvit_s1.0, hvit_s3.0);
    assert!(hvit_s1.0 < 1.0 && hvit_s3.0 < 1.0, "Wandering medics must deal no damage: {} / {}", hvit_s1.0, hvit_s3.0);

    // The whole Reed archetype branch (Incantation Medic 405) attacks with arts damage
    for inc in &["Reed the Flame Shadow", "Titi", "Amiya", "Hibiscus the Purifier", "Vendela"] {
        let (dps, _, arts) = med_5min(inc, 0);
        println!("  Incantation {} S1: DPS={:.0} arts={:.0}", inc, dps, arts);
        assert!(arts > 10.0, "Incantation branch must deal arts damage ({}): {}", inc, arts);
    }

    // Folinic (physician with explicit enemy-striking S2 "复合药剂弹片…对敌人造成…伤害")
    let folinic_s2 = med_5min("Folinic", 1);
    println!("  Folinic S2 (drug shrapnel): DPS={:.0} arts={:.0}", folinic_s2.0, folinic_s2.2);
    assert!(folinic_s2.2 > 50.0, "Folinic S2 attacks enemies and must deal arts damage: {}", folinic_s2.2);
    let folinic_s1 = med_5min("Folinic", 0);
    assert!(folinic_s1.0 < 1.0, "Folinic S1 is range/atk heal buff only: {}", folinic_s1.0);

    let mon3tr_s3 = med_5min("Mon3tr", 2);
    assert!(mon3tr_s3.1 > 500.0, "Mon3tr S3 Meltdown true damage: {}", mon3tr_s3.1);
    let nighty = med_5min("Nightingale", 2);
    assert!(nighty.0 < 1.0, "Pure healers (Nightingale) must deal NO damage: {}", nighty.0);
    let mon3tr_s2 = med_5min("Mon3tr", 1);
    assert!(mon3tr_s2.0 < 1.0, "Mon3tr S2 is pure overload healing, no damage: {}", mon3tr_s2.0);

    println!("\n=== ALL CRIT/PROC/MEDIC CHECKS PASSED ===");
}

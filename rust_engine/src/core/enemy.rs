#![allow(dead_code)]
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnemyData {
    pub id: String,
    pub name: String,
    pub tier: String, // NORMAL, ELITE, BOSS
    #[serde(default)]
    pub tier_type: Option<String>,
    pub hp: f64,
    pub atk: f64,
    pub r#def: f64,
    pub res: f64,
    pub weight: f64,
    pub move_speed: f64,
    pub attack_interval: f64,
    pub dodge_phys: f64,
    pub dodge_arts: f64,
    pub immune_stun: bool,
    pub immune_silence: bool,
    pub immune_sleep: bool,
    pub immune_freeze: bool,
    pub immune_levitate: bool,
    #[serde(default)]
    pub skill_count: Option<u64>,
    #[serde(default)]
    pub skills: Option<Vec<String>>,
    #[serde(default)]
    pub has_revive: Option<bool>,
    #[serde(default)]
    pub has_shield: Option<bool>,
    #[serde(default)]
    pub has_phase: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AverageEnemy {
    pub def: f64,
    pub res: f64,
    pub hp: f64,
    pub weight: f64,
    pub atk: f64,
    pub attack_interval: f64,
    pub dps: f64,
    pub dodge_phys: f64,
    pub dodge_arts: f64,
    pub stun_immune_ratio: f64,
    pub silence_immune_ratio: f64,
    pub freeze_immune_ratio: f64,
    pub sleep_immune_ratio: f64,
    pub levitate_immune_ratio: f64,
}

pub fn calculate_enemy_stats_for_tier(data_dir: &str, target_tier: Option<&str>) -> AverageEnemy {
    let path = Path::new(data_dir).join("Automated_Enemies.json");
    let p = if path.exists() { path } else { Path::new("./data/Automated_Enemies.json").to_path_buf() };
    
    let mut defs = Vec::new();
    let mut res_vals = Vec::new();
    let mut hps = Vec::new();
    let mut weights = Vec::new();
    let mut atks = Vec::new();
    let mut intervals = Vec::new();
    
    let mut total_count = 0.0;
    
    let mut stun_imm = 0.0;
    let mut silence_imm = 0.0;
    let mut freeze_imm = 0.0;
    let mut sleep_imm = 0.0;
    let mut levitate_imm = 0.0;
    
    let mut dodge_phys_sum = 0.0;
    let mut dodge_arts_sum = 0.0;
    
    if let Ok(file) = File::open(&p) {
        let reader = BufReader::new(file);
        if let Ok(enemies) = serde_json::from_reader::<_, Vec<EnemyData>>(reader) {
            for e in enemies {
                if let Some(req_tier) = target_tier {
                    if !e.tier.eq_ignore_ascii_case(req_tier) {
                        continue;
                    }
                } else {
                    // For composite/general, skip bosses to keep standard mob baseline or include
                    if e.tier == "BOSS" {
                        continue;
                    }
                }
                
                if e.hp <= 1.0 { continue; }
                
                defs.push(e.r#def);
                res_vals.push(e.res);
                hps.push(e.hp);
                weights.push(e.weight);
                if e.atk > 0.0 { atks.push(e.atk); }
                if e.attack_interval > 0.0 { intervals.push(e.attack_interval); }
                
                dodge_phys_sum += e.dodge_phys;
                dodge_arts_sum += e.dodge_arts;
                
                if e.immune_stun { stun_imm += 1.0; }
                if e.immune_silence { silence_imm += 1.0; }
                if e.immune_freeze { freeze_imm += 1.0; }
                if e.immune_sleep { sleep_imm += 1.0; }
                if e.immune_levitate { levitate_imm += 1.0; }
                
                total_count += 1.0;
            }
        }
    }
    
    if total_count == 0.0 {
        return match target_tier {
            Some("NORMAL") => AverageEnemy {
                def: 150.0, res: 15.0, hp: 5000.0, weight: 1.0,
                atk: 380.0, attack_interval: 2.2, dps: 172.7,
                dodge_phys: 0.0, dodge_arts: 0.0,
                stun_immune_ratio: 0.1, silence_immune_ratio: 0.2, freeze_immune_ratio: 0.1, sleep_immune_ratio: 0.1, levitate_immune_ratio: 0.1,
            },
            Some("ELITE") => AverageEnemy {
                def: 500.0, res: 30.0, hp: 15000.0, weight: 3.0,
                atk: 700.0, attack_interval: 3.25, dps: 215.4,
                dodge_phys: 0.02, dodge_arts: 0.02,
                stun_immune_ratio: 0.4, silence_immune_ratio: 0.6, freeze_immune_ratio: 0.3, sleep_immune_ratio: 0.3, levitate_immune_ratio: 0.3,
            },
            Some("BOSS") => AverageEnemy {
                def: 1000.0, res: 50.0, hp: 80000.0, weight: 6.0,
                atk: 1200.0, attack_interval: 4.0, dps: 300.0,
                dodge_phys: 0.05, dodge_arts: 0.05,
                stun_immune_ratio: 0.85, silence_immune_ratio: 0.95, freeze_immune_ratio: 0.7, sleep_immune_ratio: 0.7, levitate_immune_ratio: 0.7,
            },
            _ => AverageEnemy {
                def: 250.0, res: 20.0, hp: 8750.0, weight: 2.0,
                atk: 500.0, attack_interval: 3.0, dps: 166.7,
                dodge_phys: 0.01, dodge_arts: 0.01,
                stun_immune_ratio: 0.45, silence_immune_ratio: 0.58, freeze_immune_ratio: 0.3, sleep_immune_ratio: 0.3, levitate_immune_ratio: 0.3,
            },
        };
    }
    
    defs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    res_vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
    hps.sort_by(|a, b| a.partial_cmp(b).unwrap());
    weights.sort_by(|a, b| a.partial_cmp(b).unwrap());
    atks.sort_by(|a, b| a.partial_cmp(b).unwrap());
    intervals.sort_by(|a, b| a.partial_cmp(b).unwrap());
    
    let median = |v: &Vec<f64>| -> f64 {
        if v.is_empty() { return 0.0; }
        let mid = v.len() / 2;
        v[mid]
    };
    
    let m_def = median(&defs);
    let m_res = median(&res_vals);
    let m_hp = median(&hps);
    let m_weight = median(&weights);
    let m_atk = if atks.is_empty() { 500.0 } else { median(&atks) };
    let m_interval = if intervals.is_empty() { 3.0 } else { median(&intervals) };
    let dps = if m_interval > 0.0 { m_atk / m_interval } else { 0.0 };

    AverageEnemy {
        def: m_def,
        res: m_res,
        hp: m_hp,
        weight: m_weight,
        atk: m_atk,
        attack_interval: m_interval,
        dps,
        dodge_phys: dodge_phys_sum / total_count,
        dodge_arts: dodge_arts_sum / total_count,
        stun_immune_ratio: stun_imm / total_count,
        silence_immune_ratio: silence_imm / total_count,
        freeze_immune_ratio: freeze_imm / total_count,
        sleep_immune_ratio: sleep_imm / total_count,
        levitate_immune_ratio: levitate_imm / total_count,
    }
}

pub fn calculate_average_enemy(data_dir: &str) -> AverageEnemy {
    calculate_enemy_stats_for_tier(data_dir, None)
}

pub fn calculate_normal_enemy(data_dir: &str) -> AverageEnemy {
    calculate_enemy_stats_for_tier(data_dir, Some("NORMAL"))
}

pub fn calculate_elite_enemy(data_dir: &str) -> AverageEnemy {
    calculate_enemy_stats_for_tier(data_dir, Some("ELITE"))
}

pub fn calculate_boss_enemy(data_dir: &str) -> AverageEnemy {
    let mut b = calculate_enemy_stats_for_tier(data_dir, Some("BOSS"));
    // Ensure boss benchmark has realistic standards (at least 80k HP, 1000 DEF, 50 RES, 1200 ATK, 4.0s)
    if b.hp < 80000.0 { b.hp = 80000.0; }
    if b.def < 1000.0 { b.def = 1000.0; }
    if b.res < 50.0 { b.res = 50.0; }
    if b.atk < 1200.0 { b.atk = 1200.0; }
    if b.attack_interval < 4.0 { b.attack_interval = 4.0; }
    b.weight = b.weight.max(5.0);
    b.dps = b.atk / b.attack_interval;
    b
}

pub fn calculate_ra_enemy(_data_dir: &str) -> AverageEnemy {
    // Reclamation Algorithm (生息演算): Massive horde waves, siege beasts, and raid swarms
    AverageEnemy {
        def: 350.0,
        res: 20.0,
        hp: 12000.0,
        weight: 2.0,
        atk: 650.0,
        attack_interval: 2.5,
        dps: 260.0,
        dodge_phys: 0.0,
        dodge_arts: 0.0,
        stun_immune_ratio: 0.25,
        silence_immune_ratio: 0.35,
        freeze_immune_ratio: 0.2,
        sleep_immune_ratio: 0.2,
        levitate_immune_ratio: 0.2,
    }
}

pub fn calculate_is_enemy(_data_dir: &str) -> AverageEnemy {
    // Integrated Strategies (集成战略 / Roguelike): Relic-scaled high threat elites & bosses
    AverageEnemy {
        def: 650.0,
        res: 35.0,
        hp: 35000.0,
        weight: 4.0,
        atk: 950.0,
        attack_interval: 3.0,
        dps: 316.7,
        dodge_phys: 0.03,
        dodge_arts: 0.03,
        stun_immune_ratio: 0.6,
        silence_immune_ratio: 0.75,
        freeze_immune_ratio: 0.5,
        sleep_immune_ratio: 0.5,
        levitate_immune_ratio: 0.5,
    }
}

pub fn calculate_cc_enemy(_data_dir: &str) -> AverageEnemy {
    // Contingency Contract (危机合约 / CC): the single most demanding permanent mode in the
    // game — squads face a boss plus dense elite waves simultaneously, and players stack "Risk"
    // hazards that keep buffing enemy DEF/RES/HP/ATK well past a normal Boss encounter. Modeled
    // here as tougher than both the plain "Boss" category and Integrated Strategies (which only
    // scales a single elite/boss threat, not a whole hazard-buffed field).
    AverageEnemy {
        def: 1300.0,
        res: 60.0,
        hp: 110000.0,
        weight: 7.0,
        atk: 1600.0,
        attack_interval: 3.2,
        dps: 500.0,
        dodge_phys: 0.05,
        dodge_arts: 0.05,
        stun_immune_ratio: 0.9,
        silence_immune_ratio: 0.97,
        freeze_immune_ratio: 0.8,
        sleep_immune_ratio: 0.8,
        levitate_immune_ratio: 0.8,
    }
}

pub fn get_enemy_by_category(data_dir: &str, category: &str) -> AverageEnemy {
    match category.to_ascii_lowercase().as_str() {
        "normal" => calculate_normal_enemy(data_dir),
        "elite" => calculate_elite_enemy(data_dir),
        "boss" => calculate_boss_enemy(data_dir),
        "ra" => calculate_ra_enemy(data_dir),
        "is" => calculate_is_enemy(data_dir),
        "cc" => calculate_cc_enemy(data_dir),
        _ => calculate_average_enemy(data_dir),
    }
}


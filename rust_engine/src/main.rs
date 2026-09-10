#![recursion_limit = "256"]
#![allow(dead_code)]
use rayon::prelude::*;
use axum::{
    extract::{Path, Query},
    response::{Html, IntoResponse},
    routing::{get, post, delete},
    Json, Router,
};
use serde::Deserialize;
use tokio::net::TcpListener;
use tower_http::services::ServeDir;
use serde_json::Value;
use std::collections::HashMap;

mod core;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .nest_service("/static", ServeDir::new("../static"))
        .route("/", get(read_root))
        .route("/editor", get(operator_editor))
        .route("/tierlist", get(tierlist_view))
        .route("/enemy_tierlist", get(enemy_tierlist_view))
        .route("/api/enemy_tierlist_data", get(get_enemy_tierlist_data))
        .route("/api/tierlist/export", get(export_tierlist_csv))
        .route("/api/tierlist/export_pdfs_zip", get(download_tierlists_pdf_zip))
        .route("/api/simulate_batch", post(run_simulation_batch))
        .route("/api/tierlist_data", get(get_tierlist_data))
        .route("/api/operators", get(get_operators))
        .route("/api/simulate", post(run_simulation))
        .route("/api/operators/save", post(save_operator))
        .route("/api/operators/{op_id}", delete(delete_operator))
        .route("/enemy_editor", get(enemy_editor_view))
        .route("/api/enemies", get(get_enemies))
        .route("/api/enemies/save", post(save_enemy))
        .route("/api/enemies/{id}", delete(delete_enemy));

    // Allow overriding the port with the PORT environment variable for flexibility.
    let port = std::env::var("PORT").unwrap_or_else(|_| "8000".to_string());
    let addr = format!("127.0.0.1:{}", port);
    println!("Starting server on {}", addr);

    match TcpListener::bind(&addr).await {
        Ok(listener) => {
            // If binding succeeds, start the server normally. If serve fails, print the error and exit.
            if let Err(e) = axum::serve(listener, app).await {
                eprintln!("Server error: {}", e);
                std::process::exit(1);
            }
        }
        Err(e) => {
            // Handle the common case where the port is already in use with a friendly message.
            if e.kind() == std::io::ErrorKind::AddrInUse {
                eprintln!("Failed to bind to {}: address already in use. Stop the process using this port or set the PORT environment variable to a different port.", addr);
            } else {
                eprintln!("Failed to bind to {}: {}", addr, e);
            }
            std::process::exit(1);
        }
    }
}

async fn read_root() -> impl IntoResponse {
    let loader = core::data_loader::DataLoader::new("../data");
    let operators = loader.operators_raw.get("operators").cloned().unwrap_or(serde_json::json!([]));
    
    let mut env = minijinja::Environment::new();
    env.set_loader(minijinja::path_loader("../templates"));
    
    let html = env.get_template("dashboard.html").unwrap().render(minijinja::context! {
        operators => operators,
        version => "1"
    }).unwrap();
    Html(html)
}

#[derive(Deserialize)]
struct EditorQuery {
    op_id: Option<i32>,
}

async fn operator_editor(Query(q): Query<EditorQuery>) -> impl IntoResponse {
    let loader = core::data_loader::DataLoader::new("../data");
    let operators = loader.operators_raw.get("operators").cloned().unwrap_or(serde_json::json!([]));
    let classes = loader.classes.get("classes").cloned().unwrap_or(serde_json::json!([]));
    let mut selected_op = serde_json::Value::Null;
    if let Some(id) = q.op_id {
        if let Some(arr) = operators.as_array() {
            selected_op = arr.iter().find(|x| x.get("operator_id").and_then(|v| v.as_i64()) == Some(id as i64)).cloned().unwrap_or(serde_json::Value::Null);
        }
    }
    
    let mut env = minijinja::Environment::new();
    env.set_loader(minijinja::path_loader("../templates"));
    
    let html = env.get_template("editor.html").unwrap().render(minijinja::context! {
        operators => operators,
        classes => classes,
        selected_op => selected_op,
        version => "1"
    }).unwrap();
    Html(html)
}

async fn tierlist_view() -> impl IntoResponse {
    let mut env = minijinja::Environment::new();
    env.set_loader(minijinja::path_loader("../templates"));
    
    let html = env.get_template("tierlist.html").unwrap().render(minijinja::context! {
        version => "1"
    }).unwrap();
    Html(html)
}

async fn export_tierlist_csv(Query(q): Query<TierlistDataQuery>) -> impl IntoResponse {
    let category = q.category.as_deref().unwrap_or("general");
    let loader = core::data_loader::DataLoader::new("../data");
    let target_enemy = core::enemy::get_enemy_by_category("../data", category);
    
    let ops = loader.get_all_operator_names();
    let mut all_configs = Vec::new();
    
    use rayon::prelude::*;
    let configs: Vec<Vec<serde_json::Value>> = ops.par_iter().map(|op_name| {
        evaluate_single_operator(op_name, true, &loader, &target_enemy, category)
    }).filter_map(|x| x).collect();
    
    for mut conf_list in configs {
        all_configs.append(&mut conf_list);
    }
    
    all_configs.sort_by(|a, b| {
        let sa = b.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let sb = a.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
        sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal)
    });
    
    for (i, config) in all_configs.iter_mut().enumerate() {
        if let Some(obj) = config.as_object_mut() {
            obj.insert("rank".to_string(), serde_json::json!(i + 1));
        }
    }
    
    let mut csv = String::from("Rank,Tier,Operator,Skill,Module,Score,Phys DPS,Arts DPS,True DPS,Ele DPS,Wave TTC,Boss TTC,HPS,EHP Phys,Support Value\n");
    for config in all_configs {
        let rank = config.get("rank").and_then(|v| v.as_u64()).unwrap_or(0);
        let tier = config.get("tier").and_then(|v| v.as_str()).unwrap_or("");
        let op = config.get("operator_name").and_then(|v| v.as_str()).unwrap_or("");
        let skill = config.get("skill_name").and_then(|v| v.as_str()).unwrap_or("");
        let module = config.get("module_name").and_then(|v| v.as_str()).unwrap_or("");
        let score = config.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let phys = config.get("phys_dmg").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let arts = config.get("arts_dmg").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let true_d = config.get("true_dmg").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let ele_d = config.get("elemental_dmg").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let wave = config.get("wave_ttc").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let boss = config.get("boss_ttc").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let heal = config.get("heal").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let phys_surv = config.get("phys_surv").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let support = config.get("support").and_then(|v| v.as_f64()).unwrap_or(0.0);
        
        csv.push_str(&format!("{},{},{},{},{},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2}\n",
            rank, tier, op, skill, module, score, phys, arts, true_d, ele_d, wave, boss, heal, phys_surv, support));
    }
    
    (
        [(axum::http::header::CONTENT_DISPOSITION, "attachment; filename=\"arknights_tier_list.csv\""),
         (axum::http::header::CONTENT_TYPE, "text/csv")],
        csv,
    )
}

async fn download_tierlists_pdf_zip() -> impl IntoResponse {
    let p1 = std::path::Path::new("../data/Arknights_Tier_Lists_PDF.zip");
    let p2 = std::path::Path::new("./data/Arknights_Tier_Lists_PDF.zip");

    let p_final = if p1.exists() { p1 } else { p2 };
    if !p_final.exists() {
        let script_path = if std::path::Path::new("../Scripts/generate_tierlist_pdfs.py").exists() {
            "../Scripts/generate_tierlist_pdfs.py"
        } else if std::path::Path::new("./Scripts/generate_tierlist_pdfs.py").exists() {
            "./Scripts/generate_tierlist_pdfs.py"
        } else if std::path::Path::new("../generate_tierlist_pdfs.py").exists() {
            "../generate_tierlist_pdfs.py"
        } else {
            "./generate_tierlist_pdfs.py"
        };
        let _ = std::process::Command::new("python")
            .arg(script_path)
            .output();
    }

    if let Ok(bytes) = std::fs::read(p_final) {
        return (
            [
                (axum::http::header::CONTENT_DISPOSITION, "attachment; filename=\"Arknights_Tier_Lists_PDF.zip\""),
                (axum::http::header::CONTENT_TYPE, "application/zip"),
            ],
            bytes,
        ).into_response();
    }

    (
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        "No se pudo generar el archivo ZIP de PDFs",
    ).into_response()
}

#[derive(Deserialize)]
struct BatchSimulationRequest {
    configs: Vec<Value>,
    global_target_def: Option<f64>,
    global_target_res: Option<f64>,
}

async fn run_simulation_batch(Json(payload): Json<BatchSimulationRequest>) -> impl IntoResponse {
    let loader = core::data_loader::DataLoader::new("../data");
    let avg_enemy = core::enemy::calculate_average_enemy("../data");
    let mut results = Vec::new();
    let global_def = payload.global_target_def.unwrap_or(avg_enemy.def);
    let global_res = payload.global_target_res.unwrap_or(avg_enemy.res);
    
    for conf in payload.configs {
        if let Some(op_name) = conf.get("operator_name").and_then(|v| v.as_str()) {
            if let Some(mut op) = loader.get_operator(op_name) {
                let skill_index = conf.get("skill_index").and_then(|v| v.as_i64()).unwrap_or(-1);
                let module_index = conf.get("module_index").and_then(|v| v.as_i64()).unwrap_or(-1);
                
                if module_index >= 0 && (module_index as usize) < op.modules.len() {
                    op.active_module = Some(op.modules[module_index as usize].clone());
                } else {
                    op.active_module = None;
                }
                
                if skill_index >= 0 && (skill_index as usize) < op.skills.len() {
                    op.equipped_skill = Some(op.skills[skill_index as usize].clone());
                    op.current_state = op.equipped_skill.as_ref().unwrap().name.clone();
                } else {
                    op.equipped_skill = None;
                    op.current_state = "RAW (No Skill)".to_string();
                }
                
                let orig_skill_name = op.equipped_skill.as_ref().map(|s| s.name.clone()).unwrap_or_else(|| "None".to_string());
                let orig_module_name = op.active_module.as_ref().map(|m| m.name.clone()).unwrap_or_else(|| "None".to_string());
                let orig_state_name = op.current_state.clone();
                
                let mut target_stats = HashMap::new();
                target_stats.insert("def".to_string(), global_def);
                target_stats.insert("res".to_string(), global_res);
                target_stats.insert("atk".to_string(), avg_enemy.atk);
                target_stats.insert("attack_interval".to_string(), avg_enemy.attack_interval);
                target_stats.insert("weight".to_string(), avg_enemy.weight);
                target_stats.insert("is_boss".to_string(), if avg_enemy.hp >= 50000.0 { 1.0 } else { 0.0 });
                
                let mut env = core::simulation::SimulationEnvironment::new(op.clone(), None, Some(target_stats));
                let (dmg, heal, dp, dmg_e, heal_e, dmg_split) = env.run_5_minute_sim();
                
                let mut sampled_dmg = Vec::new();
                for (i, v) in dmg.iter().enumerate() { if i % 30 == 0 { sampled_dmg.push(*v); } }
                let mut sampled_heal = Vec::new();
                for (i, v) in heal.iter().enumerate() { if i % 30 == 0 { sampled_heal.push(*v); } }
                let mut sampled_dp = Vec::new();
                for (i, v) in dp.iter().enumerate() { if i % 30 == 0 { sampled_dp.push(*v); } }
                
                let total_dmg = dmg.last().map(|&(_, v)| v).unwrap_or(0.0);
                let total_heal = heal.last().map(|&(_, v)| v).unwrap_or(0.0);
                let total_dp = dp.last().map(|&(_, v)| v).unwrap_or(0.0);
                let max_burst_dmg = dmg_e.iter().map(|&(_, v)| v).fold(0.0, f64::max);
                let max_burst_heal = heal_e.iter().map(|&(_, v)| v).fold(0.0, f64::max);
                
                results.push(serde_json::json!({
                    "operator_name": op_name,
                    "skill_name": orig_skill_name,
                    "module_name": orig_module_name,
                    "arts_surv": op.calculate_ehp_arts(),
                    "phys_surv": op.calculate_ehp_phys(),
                    "state_name": orig_state_name,
                    "damage": sampled_dmg,
                    "healing": sampled_heal,
                    "dp": sampled_dp,
                    "damage_events": dmg_e,
                    "healing_events": heal_e,
                    "final_stats": {
                        "hp": op.final_hp(),
                        "atk": op.final_atk(),
                        "def": op.final_def(),
                        "res": op.calculate_stat("res"),
                        "aspd": op.calculate_stat("aspd"),
                        "interval": op.final_interval()
                    },
                    "summary": {
                        "total_damage": total_dmg,
                        "total_healing": total_heal,
                        "total_dp": total_dp,
                        "dps": total_dmg / 300.0,
                        "hps": total_heal / 300.0,
                        "ehp_phys": op.calculate_ehp_phys(),
                        "total_skill_damage": dmg_split.get("max_damage_per_cast").unwrap_or(&0.0),
                        "max_burst_dmg": max_burst_dmg,
                        "max_burst_heal": max_burst_heal,
                        "photo_path": op.photo_path
                    }
                }));
            }
        }
    }
    Json(serde_json::json!({ "results": results }))
}

fn evaluate_single_operator(op_name: &str, _apply_decay: bool, loader: &core::data_loader::DataLoader, avg_enemy: &core::enemy::AverageEnemy, category: &str) -> Option<Vec<Value>> {
    let dummy_op = loader.get_operator(op_name)?;
    
    let _is_support_class = dummy_op.profession == "SUPPORTER" || dummy_op.profession == "MEDIC";
    
    let skills_count = dummy_op.skills.len();
    let mut skill_indices = vec![None];
    for i in 0..skills_count {
        skill_indices.push(Some(i));
    }
    
    let mut configs = Vec::new();
    
    for skill_idx in skill_indices {
        let mut modules_to_test = vec![None];
        for (i, m) in dummy_op.modules.iter().enumerate() {
            let m_type = m.module_type.as_deref().unwrap_or("STANDARD");
            let is_ra_mod = m_type == "RA" || m.name.starts_with("RA-");
            let is_isw_mod = m_type == "ISW" || m.name.starts_with("ISW-");
            
            if category == "ra" {
                // In Reclamation Algorithm, allow STANDARD and RA modules
                if !is_isw_mod { modules_to_test.push(Some(i)); }
            } else if category == "is" {
                // In Integrated Strategies, allow STANDARD and ISW modules
                if !is_ra_mod { modules_to_test.push(Some(i)); }
            } else {
                // In standard modes (general, normal, elite, boss), allow ONLY standard modules
                if !is_ra_mod && !is_isw_mod { modules_to_test.push(Some(i)); }
            }
        }
        
        for mod_idx in modules_to_test {
            let mut op = dummy_op.clone();
            if let Some(si) = skill_idx {
                op.equipped_skill = Some(op.skills[si].clone());
                op.change_state(Some(si.to_string()));
            } else {
                op.change_state(None);
            }
            if let Some(mi) = mod_idx {
                op.active_module = Some(op.modules[mi].clone());
            } else {
                op.active_module = None;
            }
            
            let mut target_stats = std::collections::HashMap::new();
            target_stats.insert("def".to_string(), avg_enemy.def);
            target_stats.insert("res".to_string(), avg_enemy.res);
            target_stats.insert("weight".to_string(), avg_enemy.weight);
            target_stats.insert("atk".to_string(), avg_enemy.atk);
            target_stats.insert("attack_interval".to_string(), avg_enemy.attack_interval);
            let is_boss_cat = category == "boss";
            target_stats.insert("is_boss".to_string(), if is_boss_cat { 1.0 } else { 0.0 });
            
            let mut sim = core::simulation::SimulationEnvironment::new(op.clone(), None, Some(target_stats));
            
            let (_dmg_t, heal_t, _dp_t, _dmg_e, _heal_e, mut dmg_split) = sim.run_5_minute_sim();
            let wave_ttc = sim.run_wave_sim(avg_enemy.hp, avg_enemy.def, avg_enemy.res);
            let (boss_hp, boss_def, boss_res) = if is_boss_cat {
                (avg_enemy.hp, avg_enemy.def, avg_enemy.res)
            } else {
                (80000.0, 1200.0, 50.0)
            };
            let boss_ttc = sim.run_boss_sim(boss_hp, boss_def, boss_res);
            
            if let Some(phys) = dmg_split.get_mut("physical") {
                *phys *= 1.0 - avg_enemy.dodge_phys;
            }
            if let Some(arts) = dmg_split.get_mut("arts") {
                *arts *= 1.0 - avg_enemy.dodge_arts;
            }

            let arts_surv = op.calculate_ehp_arts_against(avg_enemy.atk);
            let phys_surv = op.calculate_ehp_phys_against(avg_enemy.atk);
            let base_surv = (arts_surv + phys_surv) / 2.0;
            let survivability = base_surv * (1.0 + op.calculate_stat("healing_received_bonus"));
            let hits_to_kill = op.calculate_hits_to_kill(avg_enemy.atk, avg_enemy.attack_interval, 0.20);
            let defeat_count = *dmg_split.get("defeat_count").unwrap_or(&0.0);
            let combat_uptime_factor = *dmg_split.get("combat_uptime_factor").unwrap_or(&1.0);
            
            let true_dmg = *dmg_split.get("true").unwrap_or(&0.0);
            let weakness_dmg = *dmg_split.get("weakness").unwrap_or(&0.0);
            let arts_dmg = *dmg_split.get("arts").unwrap_or(&0.0);
            let phys_dmg = *dmg_split.get("physical").unwrap_or(&0.0);
            let elemental_dmg = *dmg_split.get("elemental").unwrap_or(&0.0);
            let potential_aoe = *dmg_split.get("potential_aoe").unwrap_or(&0.0);
            let total_dp_generated = *dmg_split.get("total_dp").unwrap_or(&0.0);
            let active_heal = heal_t.last().map(|x| x.1).unwrap_or(0.0);
            
            let is_true_aoe = op.is_true_aoe();
            
            let is_team_healer = dummy_op.is_team_healer();
            let final_heal = if is_team_healer {
                active_heal + (op.calculate_stat("healing_per_second_atk_ratio") * op.final_atk() * 300.0)
            } else {
                0.0
            };
            let mut max_burst = *dmg_split.get("max_damage_per_cast").unwrap_or(&0.0);
            
            let is_ammo = op.equipped_skill.as_ref().map(|s| {
                s.buffs.iter().any(|b| b.stat.contains("trigger_time") || b.stat.contains("ammo"))
            }).unwrap_or(false);

            if let Some(s) = &op.equipped_skill {
                if !is_ammo && s.is_infinite_or_toggle() {
                    let dps = (arts_dmg + phys_dmg + true_dmg) / 300.0;
                    if max_burst > dps * 30.0 { max_burst = dps * 30.0; }
                }
            }
            
            let _total_skill_damage = max_burst;
            let mut niche_score = 0.0;
            
            let silence = op.calculate_stat("silence_duration");
            if silence > 0.0 { niche_score += 2.0; }
            let camo = op.calculate_stat("camouflage");
            if camo > 0.0 { niche_score += f64::min(3.0, camo / 5.0); }
            if op.calculate_stat("floating") > 0.0 { niche_score += 1.0; }
            if op.calculate_stat("status_resistance") > 0.0 { niche_score += 4.0; }
            if op.calculate_stat("stun_duration") > 0.0 { niche_score += 2.0; }
            if op.calculate_stat("frighten_duration") > 0.0 { niche_score += 1.5; }
            if op.calculate_stat("fear_duration") > 0.0 { niche_score += 1.5; }
            if op.calculate_stat("slow_duration") > 0.0 { niche_score += 1.0; }
            
            let pp_targets = op.calculate_stat("push_pull_targets");
            let pp_force = op.calculate_stat("push_pull_force");
            if pp_targets > 0.0 {
                let force_weight = pp_force - avg_enemy.weight + 2.0;
                niche_score += f64::max(0.5, force_weight) * pp_targets * 0.5;
            }
            
            let mut has_perm_camo = false;
            for t in &op.talents { if t.perm_camouflage { has_perm_camo = true; } }
            if op.equipped_skill.as_ref().map(|s| s.perm_camouflage).unwrap_or(false) { has_perm_camo = true; }
            if op.active_module.as_ref().map(|m| m.perm_camouflage).unwrap_or(false) { has_perm_camo = true; }
            if has_perm_camo { niche_score += 3.0; }
            
            let mut is_op_global = op.equipped_skill.as_ref().map(|s| s.is_global).unwrap_or(false) || op.active_module.as_ref().map(|m| m.is_global).unwrap_or(false);
            for t in &op.talents { if t.is_global { is_op_global = true; } }
            if is_op_global { niche_score += 3.0; }
            
            let ele_heal_score = op.calculate_stat("elemental_healing") + (op.calculate_stat("elemental_healing_atk_ratio") * op.final_atk()) + op.calculate_stat("elemental_healing_per_second") + op.calculate_stat("elemental_barrier");
            
            let mut total_dp_cost = op.calculate_stat("dp_cost");
            if !op.summons.is_empty() {
                let s_limit = op.summon_limit() as f64;
                let mut s_cost_sum = 0.0;
                for s in &op.summons {
                    s_cost_sum += s.base_stats.get("dp_cost").and_then(|v| v.as_f64()).unwrap_or(0.0);
                }
                let avg_s_cost = s_cost_sum / (op.summons.len() as f64);
                total_dp_cost += avg_s_cost * s_limit;
            }
            total_dp_cost = total_dp_cost.clamp(1.0, 50.0);
            
            let mut support_score = 0.0;
            let mut heal_phys_mitigation = 0.0;
            let mut heal_arts_mitigation = 0.0;
            let mut heal_ele_mitigation = 0.0;
            
            // Determine skill uptime
            let uptime = if let Some(s) = &op.equipped_skill {
                if s.is_infinite_or_toggle() { 
                    1.0 
                } else if s.duration + s.sp_cost > 0.0 { 
                    s.duration / (s.duration + s.sp_cost) 
                } else { 
                    1.0 
                }
            } else {
                0.0
            };

            // Check whether a buff realistically applies to allies based on stat & description
            let is_ally_buff = |st: &str, desc: &str, is_app: bool| -> bool {
                if is_app { return true; }
                match st {
                    "def" => {
                        desc.contains("友方单位防御力") || desc.contains("友军防御力") || desc.contains("目标防御力") 
                        || desc.contains("鼓舞") || desc.contains("友方单位的防御力") || desc.contains("所有友方单位防御力") 
                        || desc.contains("落点和周围8格的友方单位防御力") || (desc.contains("所有友方") && desc.contains("防御力"))
                    },
                    "magic_resistance" | "res" | "prob" | "arts_dodge" => {
                        desc.contains("友方单位法术抗性") || desc.contains("友方法术抗性") || desc.contains("目标法术抗性") 
                        || desc.contains("友方单位的法术抗性") || desc.contains("法术闪避") 
                        || (desc.contains("友方") && (desc.contains("法术抗性") || desc.contains("法术闪避")))
                    },
                    "damage_resistance" | "damage_resistance_scale" | "sanctuary" | "damage_reduction" | "phys_dmg_red" => {
                        desc.contains("友方") || desc.contains("庇护") || desc.contains("浮泡") || desc.contains("浮光泡影")
                    },
                    "ep_damage_resistance" => {
                        desc.contains("友方") || desc.contains("元素损伤")
                    },
                    "atk" | "attack_speed" | "aspd" | "sp_recovery" | "healing_received_bonus" | "max_hp" => {
                        desc.contains("友方") || desc.contains("友军") || desc.contains("全场") || desc.contains("范围内所有") || desc.contains("所有目标")
                    },
                    _ => false
                }
            };

            let mut provided_buffs = Vec::new();
            // Talents
            for t in &op.talents {
                for b in &t.buffs {
                    if is_ally_buff(&b.stat, &t.description, t.applicable_to_others) {
                        let mut cb = b.clone();
                        cb.applies_to_allies = true;
                        provided_buffs.push((cb, false)); // false = passive/always active
                    }
                }
            }
            // Skills
            if let Some(s) = &op.equipped_skill {
                for b in s.buffs.iter().chain(s.passive_buffs.iter()) {
                    if is_ally_buff(&b.stat, &s.description, s.applicable_to_others) {
                        let mut cb = b.clone();
                        cb.applies_to_allies = true;
                        provided_buffs.push((cb, true)); // true = scaled by skill uptime
                    }
                }
            }

            // Sanctuary calculation (Haruka, Quercus, Silence Alter, Tsukinogi, Nine-Colored Deer)
            let mut sanc_base = 0.0;
            let mut sanc_scale = 1.0;
            for (b, _) in &provided_buffs {
                let v = b.value.as_f64().unwrap_or(0.0).abs();
                match b.stat.as_str() {
                    "damage_resistance" | "sanctuary" | "damage_reduction" => {
                        sanc_base = f64::max(sanc_base, v);
                    },
                    "damage_resistance_scale" => {
                        sanc_scale = f64::max(sanc_scale, v);
                    },
                    _ => {}
                }
            }
            if sanc_base > 0.0 {
                let avg_sanc = f64::min(0.85, (sanc_base * (1.0 - uptime)) + (sanc_base * sanc_scale * uptime));
                let sanc_mit = avg_sanc * 1500.0 * 300.0;
                heal_phys_mitigation += sanc_mit;
                heal_arts_mitigation += sanc_mit;
                heal_ele_mitigation += sanc_mit;
                support_score += avg_sanc * 800.0;
            }

            for (b, is_skill) in &provided_buffs {
                let raw_val = b.value.as_f64().unwrap_or(0.0).abs();
                let mult = if *is_skill { uptime } else { 1.0 };
                let val = raw_val * mult;
                
                match b.stat.as_str() {
                    "atk" => {
                        if b.buff_type == "ratio" { support_score += val * 300.0; } else { support_score += val * 0.3; }
                    },
                    "aspd" | "attack_speed" => support_score += val * 3.0,
                    "hp" | "max_hp" => {
                        if b.buff_type == "ratio" { support_score += val * 300.0; } else { support_score += val * 0.3; }
                    },
                    "def" => {
                        if b.buff_type == "ratio" || raw_val <= 2.5 {
                            let mit = val * 500.0 * 3.0 * 300.0 * 0.5;
                            heal_phys_mitigation += mit;
                            support_score += val * 250.0;
                        } else {
                            let mit = val * 3.0 * 300.0 * 0.8;
                            heal_phys_mitigation += mit;
                            support_score += val * 1.0;
                        }
                    },
                    "magic_resistance" | "res" => {
                        if b.buff_type == "ratio" || raw_val <= 2.5 {
                            let mit = (val * 15.0 * 0.01) * 1500.0 * 300.0;
                            heal_arts_mitigation += mit;
                            support_score += val * 300.0;
                        } else {
                            let mit = (val * 0.01) * 1500.0 * 300.0;
                            heal_arts_mitigation += mit;
                            support_score += val * 10.0;
                        }
                    },
                    "prob" | "arts_dodge" => {
                        let mit = val * 1500.0 * 300.0;
                        heal_arts_mitigation += mit;
                        support_score += val * 500.0;
                    },
                    "ep_damage_resistance" => {
                        let boost = if op.name.contains("Eyjafjalla") && op.equipped_skill.as_ref().map(|s| s.name.contains("火山回响") || s.name.contains("Volcanic Echoes")).unwrap_or(false) {
                            5.0
                        } else {
                            1.0
                        };
                        let eff_res = f64::min(0.80, raw_val * boost * mult);
                        let mit = eff_res * 1000.0 * 300.0;
                        heal_ele_mitigation += mit;
                        support_score += eff_res * 600.0;
                    },
                    "sp_recovery" => support_score += val * 150.0,
                    "target_limit" => support_score += val * 20.0,
                    "talent_multiplier" => support_score += val * 10.0,
                    "fragile" => support_score += val * 500.0,
                    "arts_fragile" => support_score += val * 400.0,
                    "elemental_fragile" => support_score += val * 350.0,
                    "healing_received_bonus" => support_score += val * 100.0,
                    _ => {}
                }
            }

            if dummy_op.char_id.as_deref().unwrap_or("").contains("char_179_cgbird") || op.name == "Nightingale" {
                heal_arts_mitigation += 48000.0;
                support_score += 150.0;
            }
            
            let (flat_def, ratio_def) = op.target_def_debuffs();
            support_score += flat_def.abs() * 1.0;
            support_score += ratio_def.abs() * 1500.0;
            
            let (flat_res, ratio_res) = op.target_res_debuffs();
            support_score += flat_res.abs() * 20.0;
            support_score += ratio_res.abs() * 2000.0;
            
            support_score += op.fragile().abs() * 1500.0;
            support_score += op.arts_fragile().abs() * 1200.0;
            support_score += op.elemental_fragile().abs() * 1100.0;
            
            if silence > 0.0 { support_score += (silence / 5.0) * 30.0 * (1.0 - avg_enemy.silence_immune_ratio); }
            if op.calculate_stat("stun_duration") > 0.0 { support_score += (op.calculate_stat("stun_duration") / 2.0) * 25.0 * (1.0 - avg_enemy.stun_immune_ratio); }
            if op.calculate_stat("frighten_duration") > 0.0 { support_score += (op.calculate_stat("frighten_duration") / 2.0) * 30.0; }
            if op.calculate_stat("fear_duration") > 0.0 { support_score += (op.calculate_stat("fear_duration") / 3.0) * 20.0; }
            if op.calculate_stat("slow_duration") > 0.0 { support_score += (op.calculate_stat("slow_duration") / 3.0) * 20.0; }
            
            if op.is_skill_active && op.equipped_skill.as_ref().map(|s| s.is_global).unwrap_or(false) {
                support_score *= 2.0;
            }
            
            let mut eff_score = 0.0;
            if let Some(s) = &op.equipped_skill {
                let s_cost = s.sp_cost;
                let s_dur = s.duration;
                if s.is_infinite_or_toggle() {
                    eff_score = 100.0;
                } else if s_cost > 0.0 && s_dur > 0.0 {
                    let mut uptime_r = s_dur / (s_dur + s_cost);
                    if s.sp_type == "INCREASE_WHEN_ATTACK" || s.sp_type == "INCREASE_WHEN_TAKING_DAMAGE" {
                        uptime_r = s_dur / (s_dur + (s_cost * 2.5));
                    }
                    eff_score = uptime_r * 100.0;
                } else if s.sp_type == "Passive" {
                    eff_score = 100.0;
                }
            } else {
                eff_score = 100.0;
            }
            
            if op.profession == "SPECIALIST" || op.subclass_name == "Merchant" {
                if let Some(s) = &op.equipped_skill {
                    let s_dur = s.duration;
                    if s_dur > 0.0 && op.final_redeployment_time() > 0.0 {
                        eff_score = s_dur / op.final_redeployment_time() * 100.0;
                    }
                }
            }
            
            let is_wandering_medic = dummy_op.is_wandering_medic() 
                || dummy_op.sub_profession_id.to_lowercase() == "wandermedic" 
                || dummy_op.subclass_name.contains("行医") 
                || dummy_op.subclass_name == "Wandering Medic";
            
            let mut total_elemental_heal = ele_heal_score * 150.0;
            if is_wandering_medic {
                let is_eyja = op.name.contains("Eyjafjalla");
                let base_interval = if op.final_interval() > 0.0 { op.final_interval() } else { 2.85 };
                let base_ele_rate = op.final_atk() * 0.50 / base_interval;
                
                if is_eyja {
                    let s_name = op.equipped_skill.as_ref().map(|s| s.name.as_str()).unwrap_or("");
                    if s_name.contains("火山回响") || s_name.contains("Volcanic Echoes") {
                        let s3_ele_rate = (op.final_atk() * 0.60 * 5.0) / base_interval;
                        let hot_rate = 0.20 * op.final_atk() * 3.0;
                        total_elemental_heal += ((base_ele_rate * (1.0 - uptime)) + (s3_ele_rate * uptime)) * 300.0 + (hot_rate * 300.0);
                    } else if s_name.contains("无声润物") || s_name.contains("Soundless Sustenance") {
                        let s1_ele_rate = (op.final_atk() * 0.50 * 2.0) / base_interval;
                        let aura_rate = 0.08 * op.final_atk() * 3.0;
                        total_elemental_heal += (s1_ele_rate + aura_rate) * 300.0;
                    } else {
                        total_elemental_heal += base_ele_rate * 300.0 + (0.10 * op.final_atk() * 300.0);
                    }
                } else {
                    total_elemental_heal += base_ele_rate * 300.0;
                }
            }
            
            let effective_heal = if is_team_healer {
                final_heal 
                    + total_elemental_heal 
                    + heal_phys_mitigation 
                    + heal_arts_mitigation 
                    + heal_ele_mitigation
            } else {
                0.0
            };

            let mut total_score = 0.0;
            let mut base_dmg_score = arts_dmg * (1.0 + ratio_res.abs()) * (1.0 + op.arts_fragile()) * (1.0 + op.fragile());
            base_dmg_score += (phys_dmg * (1.0 + ratio_def.abs()) * (1.0 + op.fragile())) + ((flat_def.abs() / 3.0) * dmg_split.get("total_phys_hits").unwrap_or(&0.0));
            base_dmg_score += true_dmg * (1.0 + op.fragile());
            let score_elemental_dmg = elemental_dmg * (1.0 + op.elemental_fragile()) * (1.0 + op.fragile());
            base_dmg_score += score_elemental_dmg;
            
            total_score += base_dmg_score / 300.0;
            total_score += (effective_heal / 300.0) * 1.5;
            
            // Rebalanced survivability scoring: rewards effective bulk, hits-to-kill, and defender containment
            let surv_score = (survivability / 300.0) * 0.35 + (hits_to_kill.min(50.0) * 12.0);
            total_score += surv_score;

            total_score += support_score * 0.5;
            total_score += niche_score * 50.0;
            total_score += ele_heal_score * 1.5;
            
            let field_block = op.total_field_block();
            if op.is_defender() {
                if field_block < 2.0 {
                    // 1-block Defender penalty: cannot hold multi-target lanes or weight >= 2 mob swarms
                    let block_deficit = 3.0 - field_block;
                    total_score -= block_deficit * 300.0;
                } else if field_block > 3.0 {
                    total_score += (field_block - 3.0) * 150.0;
                }
            } else if field_block >= 3.0 {
                // Non-defenders with high field block (e.g. Ling summons with 3-4 block, Kal'tsit Mon3tr with 3 block, Centurions)
                total_score += (field_block - 2.0) * 120.0;
            }
            
            if is_true_aoe {
                total_score *= 1.15;
            }
            
            if category == "ra" {
                // Reclamation Algorithm values AoE coverage and swarm suppression
                total_score += potential_aoe * 0.15;
            } else if category == "is" {
                // Integrated Strategies values burst execution and CC scaling
                total_score += (max_burst / 300.0) * 0.4 + niche_score * 30.0;
            }
            
            let module_tag = if let Some(m) = &op.active_module {
                let name_up = m.name.to_uppercase();
                let m_type = m.module_type.as_deref().unwrap_or("").to_uppercase();
                if m_type == "RA" || name_up.contains("RA-") || name_up.contains("RA ") {
                    "MOD-RA".to_string()
                } else if m_type == "ISW" || m_type == "IS" || name_up.contains("ISW-") || name_up.contains("IS-") {
                    "MOD-IS".to_string()
                } else if name_up.contains("-D:") || name_up.contains("MOD-D") || name_up.starts_with("D ") {
                    "MOD-D".to_string()
                } else if name_up.contains("-Y:") || name_up.contains("MOD-Y") || name_up.contains("-Y ") || name_up.starts_with("Y ") {
                    "MOD-Y".to_string()
                } else if name_up.contains("-X:") || name_up.contains("MOD-X") || name_up.contains("-X ") || name_up.starts_with("X ") {
                    "MOD-X".to_string()
                } else {
                    "MOD".to_string()
                }
            } else {
                "NO MOD".to_string()
            };

            let is_sakiko = op.name.contains("Sakiko") || op.is_char("char_4182_oblvns");
            let skill_tag = match skill_idx {
                None => "RAW".to_string(),
                Some(0) => "S1".to_string(),
                Some(1) => {
                    if is_sakiko { "S2-1".to_string() } else { "S2".to_string() }
                },
                Some(2) => "S3".to_string(),
                Some(i) => format!("S{}", i + 1),
            };

            configs.push(serde_json::json!({
                "operator_name": op.name.clone(),
                "operator_id": dummy_op.operator_id.unwrap_or(0),
                "rarity": dummy_op.rarity,
                "rarity_bonus": match dummy_op.rarity { 6 => 20.0, 5 => 10.0, 4 => 5.0, _ => 0.0 },
                "photo_path": dummy_op.photo_path.clone(),
                "profession": dummy_op.profession.clone(),
                "subclass_name": dummy_op.subclass_name.clone(),
                "position": dummy_op.position.clone(),
                "skill_name": op.equipped_skill.as_ref().map(|s| s.name.clone()).unwrap_or_else(|| "RAW".to_string()),
                "module_name": op.active_module.as_ref().map(|m| m.name.clone()).unwrap_or_else(|| "No Module".to_string()),
                "skill_tag": skill_tag,
                "module_tag": module_tag,
                "arts_surv": arts_surv * (1.0 + op.calculate_stat("healing_received_bonus")),
                "phys_surv": phys_surv * (1.0 + op.calculate_stat("healing_received_bonus")),
                "block": field_block,
                "arts_dmg": arts_dmg,
                "score_arts_dmg": arts_dmg * (1.0 + ratio_res.abs()) * (1.0 + op.arts_fragile()) * (1.0 + op.fragile()),
                "phys_dmg": phys_dmg,
                "score_phys_dmg": (phys_dmg * (1.0 + ratio_def.abs()) * (1.0 + op.fragile())) + ((flat_def.abs() / 3.0) * dmg_split.get("total_phys_hits").unwrap_or(&0.0)),
                "elemental_dmg": elemental_dmg,
                "score_elemental_dmg": score_elemental_dmg,
                "potential_aoe": potential_aoe,
                "true_dmg": true_dmg,
                "score_true_dmg": true_dmg * (1.0 + op.fragile()),
                "weakness_dmg": weakness_dmg,
                "max_burst": max_burst,
                "heal": effective_heal,
                "heal_phys": if is_team_healer { final_heal + heal_phys_mitigation } else { 0.0 },
                "heal_arts": if is_team_healer { final_heal + heal_arts_mitigation } else { 0.0 },
                "heal_ele": if is_team_healer { final_heal + total_elemental_heal + heal_ele_mitigation } else { 0.0 },
                "raw_heal": final_heal,
                "ele_heal": total_elemental_heal,
                "dp": total_dp_generated,
                "dp_per_sec": total_dp_generated / 300.0,
                "dp_cost": total_dp_cost,
                "total_dp_cost": total_dp_cost,
                "support": support_score,
                "niche": niche_score,
                "efficiency": eff_score,
                "total_score": total_score,
                "surv": survivability,
                "hits_to_kill": hits_to_kill,
                "defeats": defeat_count,
                "uptime_pct": combat_uptime_factor * 100.0,
                "heal_phys_mitigation": heal_phys_mitigation,
                "heal_arts_mitigation": heal_arts_mitigation,
                "heal_ele_mitigation": heal_ele_mitigation,
                "wave_ttc": wave_ttc,
                "boss_ttc": boss_ttc,
            }));

            // For Sakiko S2 stance switch: generate S2-2 (Organ mode)
            if is_sakiko && skill_idx == Some(1) {
                let mut s2_2_obj = configs.last().unwrap().clone();
                if let Some(obj) = s2_2_obj.as_object_mut() {
                    obj.insert("skill_tag".to_string(), serde_json::json!("S2-2"));
                    let orig_sk = obj.get("skill_name").and_then(|v| v.as_str()).unwrap_or("满月的舞会");
                    obj.insert("skill_name".to_string(), serde_json::json!(format!("{} (Organ)", orig_sk)));
                }
                configs.push(s2_2_obj);
            }
        }
    }
    
    Some(configs)
}
#[derive(serde::Deserialize)]
struct TierlistDataQuery {
    apply_decay: Option<bool>,
    category: Option<String>,
}

async fn get_tierlist_data(Query(q): Query<TierlistDataQuery>) -> impl IntoResponse {
    let category = q.category.as_deref().unwrap_or("general");
    let loader = core::data_loader::DataLoader::new("../data");
    let apply_decay = q.apply_decay.unwrap_or(true);
    let target_enemy = core::enemy::get_enemy_by_category("../data", category);
    let raw_ops = loader.operators_raw.get("operators").and_then(|x| x.as_array());
    if raw_ops.is_none() {
        return Json(serde_json::json!({ "general": [], "detailed": [], "enemy_stats": target_enemy, "category": category }));
    }
    let raw_ops = raw_ops.unwrap();
    let mut all_configs: Vec<_> = raw_ops.par_iter().filter_map(|op_val| {
        if let Some(op_name) = op_val.get("name").and_then(|v| v.as_str()) {
            if let Some(mut configs) = evaluate_single_operator(op_name, apply_decay, &loader, &target_enemy, category) {
                for c in &mut configs {
                    if let Some(obj) = c.as_object_mut() {
                        let arts = obj.get("arts_dmg").and_then(|v| v.as_f64()).unwrap_or(0.0);
                        let phys = obj.get("phys_dmg").and_then(|v| v.as_f64()).unwrap_or(0.0);
                        let true_d = obj.get("true_dmg").and_then(|v| v.as_f64()).unwrap_or(0.0);
                        let ele_d = obj.get("elemental_dmg").and_then(|v| v.as_f64()).unwrap_or(0.0);
                        obj.insert("total_dmg".to_string(), serde_json::json!(arts + phys + true_d + ele_d));
                    }
                }
                return Some(configs);
            }
        }
        None
    }).flatten().collect();
    
    if category == "dp" {
        all_configs.retain(|c| {
            let dp = c.get("dp").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let prof = c.get("profession").and_then(|v| v.as_str()).unwrap_or("");
            dp > 1.0 || prof == "PIONEER" || prof == "VANGUARD"
        });
    }

    if all_configs.is_empty() {
        return Json(serde_json::json!({ "general": [], "detailed": [] }));
    }
    
    let keys = vec!["niche", "support", "score_true_dmg", "score_elemental_dmg", "weakness_dmg", "ele_heal", "dp", "heal", "surv", "block", "score_arts_dmg", "score_phys_dmg", "potential_aoe", "efficiency", "max_burst"];
    
    let n_total = all_configs.len() as f64;
    let mut stat_stats = HashMap::new();
    
    for key in &keys {
        let mut possessors = Vec::new();
        for c in &all_configs {
            if let Some(val) = c.get(*key).and_then(|v| v.as_f64()) {
                if val.abs() > 0.001 { possessors.push(val); }
            }
        }
        if possessors.is_empty() {
            stat_stats.insert(key.to_string(), (0.0, 1.0));
        } else {
            let p = (possessors.len() as f64) / n_total;
            let avg = possessors.iter().sum::<f64>() / (possessors.len() as f64);
            stat_stats.insert(key.to_string(), (p, avg));
        }
    }
    
    let mut total_dp_sum = 0.0;
    for c in &all_configs {
        total_dp_sum += c.get("total_dp_cost").and_then(|v| v.as_f64()).unwrap_or(0.0);
    }
    let avg_dp_cost = if n_total > 0.0 { total_dp_sum / n_total } else { 1.0 };
    
    for c in &mut all_configs {
        let mut base_score = 0.0;
        if let Some(obj) = c.as_object_mut() {
            for key in &keys {
                let val = obj.get(*key).and_then(|v| v.as_f64()).unwrap_or(0.0);
                if val.abs() < 0.001 { continue; }
                
                let (p, avg) = stat_stats.get(*key).unwrap();
                let mut w_rarity = 2.0;
                let mut w_perf = 8.0;
                
                if category == "dp" {
                    if *key == "dp" {
                        w_rarity = 4.0;
                        w_perf = 36.0;
                    } else if *key == "surv" {
                        w_perf = 8.0;
                    } else {
                        w_perf = 3.0;
                    }
                } else if *key == "niche" {
                    w_rarity = 5.0;
                    w_perf = 5.0;
                } else if *key == "heal" {
                    w_rarity = 4.0;
                    w_perf = 24.0;
                } else if *key == "support" {
                    w_rarity = 4.0;
                    w_perf = 16.0;
                }
                
                let rarity_score = w_rarity * (1.0 - p);
                let perf_score = w_perf * (val / avg);
                base_score += rarity_score + perf_score;
            }
            
            let op_total_dp = obj.get("total_dp_cost").and_then(|v| v.as_f64()).unwrap_or(avg_dp_cost).max(1.0);
            let cost_penalty_ratio = if avg_dp_cost > 0.0 { (op_total_dp / avg_dp_cost).clamp(0.1, 4.0) } else { 1.0 };
            let dp_penalty_value = 25.0 * cost_penalty_ratio;
            base_score -= dp_penalty_value;
            
            let rarity_bonus = obj.get("rarity_bonus").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let score = f64::max(0.0, base_score + rarity_bonus);
            obj.insert("score".to_string(), serde_json::json!(score));
        }
    }
    
    all_configs.sort_by(|a, b| {
        let sa = a.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let sb = b.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
        sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
    });
    
    let n_detailed = all_configs.len();
    for (i, c) in all_configs.iter_mut().enumerate() {
        if let Some(obj) = c.as_object_mut() {
            obj.insert("rank".to_string(), serde_json::json!(i + 1));
            let pct = ((i + 1) as f64) / (n_detailed as f64);
            let tier = if pct <= 0.025 { "OP" }
            else if pct <= 0.11 { "S" }
            else if pct <= 0.27 { "A" }
            else if pct <= 0.49 { "B" }
            else if pct <= 0.71 { "C" }
            else if pct <= 0.86 { "D" }
            else if pct <= 0.95 { "E" }
            else { "F" };
            obj.insert("tier".to_string(), serde_json::json!(tier));
        }
    }
    
    let mut op_groups: HashMap<String, Vec<Value>> = HashMap::new();
    for c in &all_configs {
        if let Some(name) = c.get("operator_name").and_then(|v| v.as_str()) {
            op_groups.entry(name.to_string()).or_default().push(c.clone());
        }
    }
    
    let mut general_tierlist = Vec::new();
    for (_op_name, configs) in op_groups {
        let best = configs.iter().max_by(|a, b| {
            let sa = a.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let sb = b.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
            sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal)
        }).unwrap();
        
        let mut gen_entry = best.clone();
        if let Some(obj) = gen_entry.as_object_mut() {
            obj.insert("skill_name".to_string(), serde_json::json!("Peak Performance"));
            obj.insert("module_name".to_string(), serde_json::json!("Peak Performance"));
        }
        general_tierlist.push(gen_entry);
    }
    
    general_tierlist.sort_by(|a, b| {
        let sa = a.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let sb = b.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
        sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
    });
    
    let n_gen = general_tierlist.len();
    for (i, c) in general_tierlist.iter_mut().enumerate() {
        if let Some(obj) = c.as_object_mut() {
            obj.insert("rank".to_string(), serde_json::json!(i + 1));
            let pct = ((i + 1) as f64) / (n_gen as f64);
            let tier = if pct <= 0.025 { "OP" }
            else if pct <= 0.11 { "S" }
            else if pct <= 0.27 { "A" }
            else if pct <= 0.49 { "B" }
            else if pct <= 0.71 { "C" }
            else if pct <= 0.86 { "D" }
            else if pct <= 0.95 { "E" }
            else { "F" };
            obj.insert("tier".to_string(), serde_json::json!(tier));
        }
    }
    
    Json(serde_json::json!({
        "general": general_tierlist,
        "detailed": all_configs,
        "enemy_stats": target_enemy,
        "category": category
    }))
}

async fn get_operators() -> impl IntoResponse {
    let loader = core::data_loader::DataLoader::new("../data");
    Json(loader.operators_raw)
}

#[derive(Deserialize)]
struct SimulateRequest {
    operator_name: String,
    skill_index: Option<usize>,
    target_def: Option<f64>,
    target_res: Option<f64>,
    apply_potentials: Option<bool>,
    apply_trust: Option<bool>,
    module_index: Option<isize>,
}

async fn run_simulation(Json(payload): Json<SimulateRequest>) -> impl IntoResponse {
    let loader = core::data_loader::DataLoader::new("../data");
    
    let mut op = loader.get_operator(&payload.operator_name)
        .unwrap_or_else(|| core::models::Operator { name: payload.operator_name.clone(), ..Default::default() });
        
    if let Some(idx) = payload.skill_index {
        if idx < op.skills.len() {
            op.equipped_skill = Some(op.skills[idx].clone());
            op.current_state = op.skills[idx].name.clone();
        }
    }
    
    let orig_skill_name = op.equipped_skill.as_ref().map(|s| s.name.clone()).unwrap_or_else(|| "None".to_string());
    let state_name = op.current_state.clone();
    
    let avg_enemy = core::enemy::calculate_average_enemy("../data");
    let mut target_stats = HashMap::new();
    let def = payload.target_def.unwrap_or(avg_enemy.def);
    let res = payload.target_res.unwrap_or(avg_enemy.res);
    target_stats.insert("def".to_string(), def);
    target_stats.insert("res".to_string(), res);
    target_stats.insert("atk".to_string(), avg_enemy.atk);
    target_stats.insert("attack_interval".to_string(), avg_enemy.attack_interval);
    target_stats.insert("weight".to_string(), avg_enemy.weight);
    target_stats.insert("is_boss".to_string(), 0.0);
    
    let mut env = core::simulation::SimulationEnvironment::new(
        op.clone(),
        None,
        Some(target_stats)
    );
    
    let (dmg, heal, dp, _, _, dmg_split) = env.run_5_minute_sim();
    
    let total_dmg = dmg.last().map(|&(_, v)| v).unwrap_or(0.0);
    let total_heal = heal.last().map(|&(_, v)| v).unwrap_or(0.0);
    let phys_surv = op.calculate_ehp_phys_against(avg_enemy.atk);
    let arts_surv = op.calculate_ehp_arts_against(avg_enemy.atk);
    
    let summary = serde_json::json!({
        "total_damage": total_dmg,
        "total_healing": total_heal,
        "total_dp": 0.0,
        "dps": total_dmg / 300.0,
        "hps": total_heal / 300.0,
        "ehp_phys": phys_surv,
        "total_skill_damage": dmg_split.get("max_damage_per_cast").unwrap_or(&0.0),
        "photo_path": ""
    });
    
    let res = serde_json::json!({
        "operator_name": payload.operator_name,
        "skill_name": orig_skill_name,
        "module_name": "None",
        "arts_surv": arts_surv,
        "phys_surv": phys_surv,
        "state_name": state_name,
        "damage": dmg,
        "healing": heal,
        "dp": dp,
        "dps_instant": [],
        "summary": summary
    });
    
    Json(res)
}

async fn save_operator(mut multipart: axum::extract::Multipart) -> impl IntoResponse {
    let mut op_data: Option<Value> = None;
    let mut photo_bytes: Option<axum::body::Bytes> = None;
    let mut photo_filename: Option<String> = None;

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let name = field.name().unwrap_or("").to_string();
        if name == "data" {
            if let Ok(bytes) = field.bytes().await {
                let text = String::from_utf8_lossy(&bytes);
                op_data = serde_json::from_str(&text).ok();
            }
        } else if name == "photo" {
            if let Some(filename) = field.file_name().map(|s| s.to_string()) {
                if !filename.is_empty() {
                    photo_filename = Some(filename);
                    photo_bytes = field.bytes().await.ok();
                }
            }
        }
    }

    if let Some(mut data) = op_data {
        if let Some(filename) = photo_filename {
            if let Some(bytes) = photo_bytes {
                let path = format!("../static/images/{}", filename);
                let _ = std::fs::write(&path, bytes);
                if let Some(obj) = data.as_object_mut() {
                    obj.insert("photo_path".to_string(), serde_json::json!(filename));
                }
            }
        }
        let mut loader = core::data_loader::DataLoader::new("../data");
        let op_id = data.get("operator_id").and_then(|v| v.as_i64());
        if let Some(id) = op_id {
            let _ = loader.update_operator(id, data);
        } else {
            let _ = loader.add_operator(data);
        }
        return (axum::http::StatusCode::OK, Json(serde_json::json!({"message": "Operator saved successfully"})));
    }
    
    (axum::http::StatusCode::BAD_REQUEST, Json(serde_json::json!({"detail": "Invalid payload"})))
}

async fn delete_operator(Path(op_id): Path<i64>) -> impl IntoResponse {
    let mut loader = core::data_loader::DataLoader::new("../data");
    if loader.delete_operator(op_id).unwrap_or(false) {
        (axum::http::StatusCode::OK, Json(serde_json::json!({"message": "Operator deleted successfully"})))
    } else {
        (axum::http::StatusCode::NOT_FOUND, Json(serde_json::json!({"detail": "Operator not found"})))
    }
}

#[derive(Deserialize)]
struct EnemyTierlistQuery {
    category: Option<String>,
}

async fn enemy_tierlist_view() -> impl IntoResponse {
    let mut env = minijinja::Environment::new();
    env.set_loader(minijinja::path_loader("../templates"));
    
    let html = env.get_template("enemy_tierlist.html").unwrap().render(minijinja::context! {
        version => "1"
    }).unwrap();
    Html(html)
}

async fn get_enemy_tierlist_data(Query(params): Query<EnemyTierlistQuery>) -> impl IntoResponse {
    let mut category = params.category.unwrap_or_else(|| "all".to_string()).to_lowercase();
    let p = std::path::Path::new("../data/Automated_Enemies.json");
    let path = if p.exists() { p } else { std::path::Path::new("./data/Automated_Enemies.json") };
    
    let mut enemies: Vec<Value> = Vec::new();
    if let Ok(file) = std::fs::File::open(path) {
        let reader = std::io::BufReader::new(file);
        if let Ok(list) = serde_json::from_reader::<_, Vec<Value>>(reader) {
            enemies = list;
        }
    if category == "general" {
        category = "all".to_string();
    }
    }

    if category != "all" {
        enemies.retain(|e| {
            let t = e.get("tier_type")
                .or_else(|| e.get("tier"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_lowercase();
            t == category
        });
    }

    let mut enriched = Vec::new();
    for e in enemies {
        let mut obj = match e.as_object() {
            Some(o) => o.clone(),
            None => continue,
        };

        let orig_tier = obj.get("tier_type")
            .or_else(|| obj.get("tier"))
            .and_then(|v| v.as_str())
            .unwrap_or("NORMAL")
            .to_uppercase();
        obj.insert("tier_type".to_string(), serde_json::json!(orig_tier));
        obj.insert("category".to_string(), serde_json::json!(orig_tier));

        let hp = obj.get("hp").and_then(|v| v.as_f64()).unwrap_or(1000.0).max(1.0);
        let def = obj.get("def").and_then(|v| v.as_f64()).unwrap_or(0.0).max(0.0);
        let res = obj.get("res").and_then(|v| v.as_f64()).unwrap_or(0.0).clamp(0.0, 95.0);
        let atk = obj.get("atk").and_then(|v| v.as_f64()).unwrap_or(100.0).max(0.0);
        let interval = obj.get("attack_interval").and_then(|v| v.as_f64()).unwrap_or(2.5).max(0.2);
        let weight = obj.get("weight").and_then(|v| v.as_f64()).unwrap_or(1.0).max(0.0);
        let dp = obj.get("dodge_phys").and_then(|v| v.as_f64()).unwrap_or(0.0).clamp(0.0, 0.9);
        let da = obj.get("dodge_arts").and_then(|v| v.as_f64()).unwrap_or(0.0).clamp(0.0, 0.9);
        let avg_dodge = (dp + da) / 2.0;

        let skill_count = obj.get("skill_count").and_then(|v| v.as_u64()).unwrap_or(0) as f64;
        let has_revive = obj.get("has_revive").and_then(|v| v.as_bool()).unwrap_or(false);
        let has_shield = obj.get("has_shield").and_then(|v| v.as_bool()).unwrap_or(false);
        let has_phase = obj.get("has_phase").and_then(|v| v.as_bool()).unwrap_or(false);

        // Effective ATK logic:
        // Bosses/elites with 0 direct auto-attack rely on skills, global auras or blade beams
        let effective_atk = if atk <= 100.0 {
            if orig_tier == "BOSS" {
                1500.0
            } else if orig_tier == "ELITE" {
                600.0
            } else {
                atk
            }
        } else {
            atk
        };

        let base_dps = effective_atk / interval;
        let ability_dps = if orig_tier == "BOSS" && skill_count > 0.0 {
            (skill_count * 150.0).min(1000.0)
        } else if orig_tier == "ELITE" && skill_count > 0.0 {
            (skill_count * 60.0).min(300.0)
        } else {
            0.0
        };
        let dps = base_dps + ability_dps;

        let mut ehp = (hp * (1.0 + (def / 400.0) + (res / 40.0))) / (1.0 - avg_dodge).max(0.1);
        if has_revive { ehp *= 1.8; }
        if has_phase { ehp *= 1.4; }
        if has_shield { ehp *= 1.5; }

        let mut imm_mult = 1.0;
        if obj.get("immune_stun").and_then(|v| v.as_bool()).unwrap_or(false) { imm_mult += 0.20; }
        if obj.get("immune_silence").and_then(|v| v.as_bool()).unwrap_or(false) { imm_mult += 0.15; }
        if obj.get("immune_freeze").and_then(|v| v.as_bool()).unwrap_or(false) { imm_mult += 0.15; }
        if obj.get("immune_sleep").and_then(|v| v.as_bool()).unwrap_or(false) { imm_mult += 0.10; }
        if obj.get("immune_levitate").and_then(|v| v.as_bool()).unwrap_or(false) { imm_mult += 0.10; }
        imm_mult += weight * 0.05;

        let threat_score = (ehp.sqrt() * dps.sqrt() * imm_mult / 10.0).round();

        obj.insert("ehp".to_string(), serde_json::json!(ehp.round()));
        obj.insert("dps".to_string(), serde_json::json!(dps.round()));
        obj.insert("threat_score".to_string(), serde_json::json!(threat_score));
        enriched.push(Value::Object(obj));
    }

    enriched.sort_by(|a, b| {
        let sa = a.get("threat_score").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let sb = b.get("threat_score").and_then(|v| v.as_f64()).unwrap_or(0.0);
        sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
    });

    let n = enriched.len();
    for (i, e) in enriched.iter_mut().enumerate() {
        if let Some(obj) = e.as_object_mut() {
            obj.insert("rank".to_string(), serde_json::json!(i + 1));
            let pct = ((i + 1) as f64) / (n.max(1) as f64);
            let tier = if pct <= 0.03 { "OP" }
            else if pct <= 0.12 { "S" }
            else if pct <= 0.28 { "A" }
            else if pct <= 0.50 { "B" }
            else if pct <= 0.72 { "C" }
            else if pct <= 0.86 { "D" }
            else if pct <= 0.95 { "E" }
            else { "F" };
            obj.insert("tier".to_string(), serde_json::json!(tier));
        }
    }

    Json(serde_json::json!({
        "enemies": enriched,
        "category": category,
        "total_count": n
    }))
}

async fn enemy_editor_view() -> impl IntoResponse {
    let mut env = minijinja::Environment::new();
    env.set_loader(minijinja::path_loader("../templates"));
    
    let html = env.get_template("enemy_editor.html").unwrap().render(minijinja::context! {
        version => "1"
    }).unwrap();
    Html(html)
}

async fn get_enemies() -> impl IntoResponse {
    let p = std::path::Path::new("../data/Automated_Enemies.json");
    let path = if p.exists() { p } else { std::path::Path::new("./data/Automated_Enemies.json") };
    
    if let Ok(file) = std::fs::File::open(path) {
        let reader = std::io::BufReader::new(file);
        if let Ok(enemies) = serde_json::from_reader::<_, Value>(reader) {
            return Json(enemies);
        }
    }
    Json(serde_json::json!([]))
}

async fn save_enemy(Json(payload): Json<core::enemy::EnemyData>) -> impl IntoResponse {
    let p = std::path::Path::new("../data/Automated_Enemies.json");
    let path = if p.exists() { p } else { std::path::Path::new("./data/Automated_Enemies.json") };
    
    let mut enemies: Vec<core::enemy::EnemyData> = Vec::new();
    if let Ok(file) = std::fs::File::open(path) {
        let reader = std::io::BufReader::new(file);
        if let Ok(list) = serde_json::from_reader(reader) {
            enemies = list;
        }
    }
    
    if let Some(pos) = enemies.iter().position(|e| e.id == payload.id) {
        enemies[pos] = payload;
    } else {
        enemies.push(payload);
    }
    
    if let Ok(file) = std::fs::File::create(path) {
        let writer = std::io::BufWriter::new(file);
        if serde_json::to_writer_pretty(writer, &enemies).is_ok() {
            if path == std::path::Path::new("../data/Automated_Enemies.json") {
                if let Ok(file_local) = std::fs::File::create("./data/Automated_Enemies.json") {
                    let _ = serde_json::to_writer_pretty(std::io::BufWriter::new(file_local), &enemies);
                }
            }
            return (axum::http::StatusCode::OK, Json(serde_json::json!({ "message": "Enemy saved successfully" })));
        }
    }
    
    (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "detail": "Failed to save enemy" })))
}

async fn delete_enemy(Path(enemy_id): Path<String>) -> impl IntoResponse {
    let p = std::path::Path::new("../data/Automated_Enemies.json");
    let path = if p.exists() { p } else { std::path::Path::new("./data/Automated_Enemies.json") };
    
    let mut enemies: Vec<core::enemy::EnemyData> = Vec::new();
    if let Ok(file) = std::fs::File::open(path) {
        let reader = std::io::BufReader::new(file);
        if let Ok(list) = serde_json::from_reader(reader) {
            enemies = list;
        }
    }
    
    let orig_len = enemies.len();
    enemies.retain(|e| e.id != enemy_id);
    
    if enemies.len() < orig_len {
        if let Ok(file) = std::fs::File::create(path) {
            let writer = std::io::BufWriter::new(file);
            let _ = serde_json::to_writer_pretty(writer, &enemies);
            if path == std::path::Path::new("../data/Automated_Enemies.json") {
                if let Ok(file_local) = std::fs::File::create("./data/Automated_Enemies.json") {
                    let _ = serde_json::to_writer_pretty(std::io::BufWriter::new(file_local), &enemies);
                }
            }
            return (axum::http::StatusCode::OK, Json(serde_json::json!({ "message": "Enemy deleted successfully" })));
        }
    }
    
    (axum::http::StatusCode::NOT_FOUND, Json(serde_json::json!({ "detail": "Enemy not found" })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_healing_rankings() {
        let loader = core::data_loader::DataLoader::new("../data");
        let avg_enemy = core::enemy::calculate_average_enemy("../data");
        let raw_ops = loader.operators_raw.get("operators").and_then(|x| x.as_array()).unwrap();
        
        let mut results = Vec::new();
        for op_val in raw_ops {
            if let Some(op_name) = op_val.get("name").and_then(|v| v.as_str()) {
                if let Some(configs) = evaluate_single_operator(op_name, true, &loader, &avg_enemy, "general") {
                    for c in configs {
                        let heal = c.get("heal").and_then(|v| v.as_f64()).unwrap_or(0.0);
                        let score = c.get("total_score").and_then(|v| v.as_f64()).unwrap_or(0.0);
                        let skill = c.get("skill_name").and_then(|v| v.as_str()).unwrap_or("");
                        let module = c.get("module_name").and_then(|v| v.as_str()).unwrap_or("");
                        let prof = c.get("profession").and_then(|v| v.as_str()).unwrap_or("");
                        let sub = c.get("subclass_name").and_then(|v| v.as_str()).unwrap_or("");
                        results.push((op_name.to_string(), skill.to_string(), module.to_string(), prof.to_string(), sub.to_string(), heal, score));
                    }
                }
            }
        }

        results.sort_by(|a, b| b.5.partial_cmp(&a.5).unwrap());

        println!("\n=== TOP 25 BY HEAL ===");
        for (i, r) in results.iter().take(25).enumerate() {
            println!("{}. {} ({}/{}) - Skill: {}, Mod: {} => HEAL: {:.1}, SCORE: {:.1}", i+1, r.0, r.3, r.4, r.1, r.2, r.5, r.6);
        }

        println!("\n=== SPECIFIC TARGETS ===");
        for name in &["Haruka", "EyjafjallaAlter", "Mon3tr", "Nightingale"] {
            for r in results.iter().filter(|r| r.0 == *name) {
                println!("{} - Skill: {}, Mod: {} => HEAL: {:.1}, SCORE: {:.1}", r.0, r.1, r.2, r.5, r.6);
            }
        }

        let max_heal_of = |target: &str| -> f64 {
            results.iter()
                .filter(|r| r.0 == target)
                .map(|r| r.5)
                .fold(0.0, f64::max)
        };

        let haruka_h = max_heal_of("Haruka");
        let eyja_h = max_heal_of("EyjafjallaAlter");
        let mon3tr_h = max_heal_of("Mon3tr");
        let nightingale_h = max_heal_of("Nightingale");

        assert!(haruka_h > eyja_h, "Haruka ({}) should be > EyjafjallaAlter ({})", haruka_h, eyja_h);
        assert!(eyja_h > mon3tr_h, "EyjafjallaAlter ({}) should be > Mon3tr ({})", eyja_h, mon3tr_h);
        assert!(mon3tr_h > nightingale_h, "Mon3tr ({}) should be > Nightingale ({})", mon3tr_h, nightingale_h);
        assert!(nightingale_h > 200_000.0, "Nightingale should have strong multi-target healing");
    }

    #[test]
    fn test_damage_rankings() {
        let loader = core::data_loader::DataLoader::new("../data");
        let avg_enemy = core::enemy::calculate_average_enemy("../data");
        let raw_ops = loader.operators_raw.get("operators").and_then(|x| x.as_array()).unwrap();
        
        let mut phys_results = Vec::new();
        let mut arts_results = Vec::new();
        let mut ele_results = Vec::new();
        let mut true_results = Vec::new();

        for op_val in raw_ops {
            if let Some(op_name) = op_val.get("name").and_then(|v| v.as_str()) {
                if let Some(configs) = evaluate_single_operator(op_name, true, &loader, &avg_enemy) {
                    for c in configs {
                        let phys = c.get("phys_dmg").and_then(|v| v.as_f64()).unwrap_or(0.0);
                        let arts = c.get("arts_dmg").and_then(|v| v.as_f64()).unwrap_or(0.0);
                        let ele = c.get("elemental_dmg").and_then(|v| v.as_f64()).unwrap_or(0.0);
                        let true_d = c.get("true_dmg").and_then(|v| v.as_f64()).unwrap_or(0.0);
                        let score_phys = c.get("score_phys_dmg").and_then(|v| v.as_f64()).unwrap_or(0.0);
                        let score_arts = c.get("score_arts_dmg").and_then(|v| v.as_f64()).unwrap_or(0.0);
                        let score_ele = c.get("score_elemental_dmg").and_then(|v| v.as_f64()).unwrap_or(0.0);
                        let score = c.get("total_score").and_then(|v| v.as_f64()).unwrap_or(0.0);
                        let skill = c.get("skill_name").and_then(|v| v.as_str()).unwrap_or("");
                        let module = c.get("module_name").and_then(|v| v.as_str()).unwrap_or("");
                        let prof = c.get("profession").and_then(|v| v.as_str()).unwrap_or("");
                        let sub = c.get("subclass_name").and_then(|v| v.as_str()).unwrap_or("");
                        
                        phys_results.push((op_name.to_string(), skill.to_string(), module.to_string(), prof.to_string(), sub.to_string(), phys, score_phys, score));
                        arts_results.push((op_name.to_string(), skill.to_string(), module.to_string(), prof.to_string(), sub.to_string(), arts, score_arts, score));
                        ele_results.push((op_name.to_string(), skill.to_string(), module.to_string(), prof.to_string(), sub.to_string(), ele, score_ele, score));
                        true_results.push((op_name.to_string(), skill.to_string(), module.to_string(), prof.to_string(), sub.to_string(), true_d, score));
                    }
                }
            }
        }

        phys_results.sort_by(|a, b| b.5.partial_cmp(&a.5).unwrap());
        println!("\n=== TOP 20 BY PHYSICAL DAMAGE ===");
        for (i, r) in phys_results.iter().take(20).enumerate() {
            println!("{}. {} ({}/{}) - Skill: {} => PHYS: {:.1}, SCORE_PHYS: {:.1}, TOTAL: {:.1}", i+1, r.0, r.3, r.4, r.1, r.5, r.6, r.7);
        }

        arts_results.sort_by(|a, b| b.5.partial_cmp(&a.5).unwrap());
        println!("\n=== TOP 20 BY ARTS DAMAGE ===");
        for (i, r) in arts_results.iter().take(20).enumerate() {
            println!("{}. {} ({}/{}) - Skill: {} => ARTS: {:.1}, SCORE_ARTS: {:.1}, TOTAL: {:.1}", i+1, r.0, r.3, r.4, r.1, r.5, r.6, r.7);
        }

        ele_results.sort_by(|a, b| b.5.partial_cmp(&a.5).unwrap());
        println!("\n=== TOP 20 BY ELEMENTAL DAMAGE ===");
        for (i, r) in ele_results.iter().take(20).enumerate() {
            println!("{}. {} ({}/{}) - Skill: {} => ELE: {:.1}, SCORE_ELE: {:.1}, TOTAL: {:.1}", i+1, r.0, r.3, r.4, r.1, r.5, r.6, r.7);
        }

        println!("\n=== TARGET PHYSICAL OPERATORS ===");
        for name in &["Ray", "Exusiai", "Wisadel"] {
            for r in phys_results.iter().filter(|r| r.0 == *name) {
                println!("{} - Skill: {} => PHYS: {:.1}, SCORE_PHYS: {:.1}", r.0, r.1, r.5, r.6);
            }
        }

        println!("\n=== TARGET ARTS OPERATORS ===");
        for name in &["LapplandAlter", "PramanixAlter", "Logos"] {
            for r in arts_results.iter().filter(|r| r.0 == *name) {
                println!("{} - Skill: {} => ARTS: {:.1}, SCORE_ARTS: {:.1}", r.0, r.1, r.5, r.6);
            }
        }

        println!("\n=== TARGET ELEMENTAL OPERATORS ===");
        for name in &["BlazeAlter", "Tragodia", "Mantra", "Logos"] {
            for r in ele_results.iter().filter(|r| r.0 == *name) {
                println!("{} - Skill: {} => ELE: {:.1}, SCORE_ELE: {:.1}", r.0, r.1, r.5, r.6);
            }
        }

        let max_phys_of = |target: &str| -> f64 {
            phys_results.iter().filter(|r| r.0 == target).map(|r| r.5).fold(0.0, f64::max)
        };
        let max_arts_of = |target: &str| -> f64 {
            arts_results.iter().filter(|r| r.0 == target).map(|r| r.5).fold(0.0, f64::max)
        };
        let max_ele_of = |target: &str| -> f64 {
            ele_results.iter().filter(|r| r.0 == target).map(|r| r.5).fold(0.0, f64::max)
        };

        // Assert Physical hierarchy: Ray > Exusiai > Wis'adel
        let ray_p = max_phys_of("Ray");
        let exu_p = max_phys_of("Exusiai");
        let wis_p = max_phys_of("Wisadel");
        assert!(ray_p > exu_p, "Ray ({}) must be > Exusiai ({})", ray_p, exu_p);
        assert!(exu_p > wis_p, "Exusiai ({}) must be > Wisadel ({})", exu_p, wis_p);

        // Assert Arts hierarchy: LapplandAlter > PramanixAlter > Logos
        let lapp_a = max_arts_of("LapplandAlter");
        let pram_a = max_arts_of("PramanixAlter");
        let logos_a = max_arts_of("Logos");
        assert!(lapp_a > pram_a, "LapplandAlter ({}) must be > PramanixAlter ({})", lapp_a, pram_a);
        assert!(pram_a > logos_a, "PramanixAlter ({}) must be > Logos ({})", pram_a, logos_a);

        // Assert Elemental hierarchy: BlazeAlter > Tragodia > Mantra > Logos
        let blaze_e = max_ele_of("BlazeAlter");
        let trag_e = max_ele_of("Tragodia");
        let mantra_e = max_ele_of("Mantra");
        let logos_e = max_ele_of("Logos");
        assert!(blaze_e > trag_e, "BlazeAlter ({}) must be > Tragodia ({})", blaze_e, trag_e);
        assert!(trag_e > mantra_e, "Tragodia ({}) must be > Mantra ({})", trag_e, mantra_e);
        assert!(mantra_e > logos_e, "Mantra ({}) must be > Logos ({})", mantra_e, logos_e);
    }
}


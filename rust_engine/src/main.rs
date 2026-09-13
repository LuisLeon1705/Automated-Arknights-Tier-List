#![recursion_limit = "256"]
#![allow(dead_code)]
use rayon::prelude::*;
use axum::{
    extract::{Path, Query, State},
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

#[derive(Clone)]
struct TeamTierlistCache {
    teams: Vec<Value>,
    data_signature: u64,
}

#[derive(Clone)]
struct AppState {
    team_tierlist_cache: std::sync::Arc<tokio::sync::RwLock<Option<TeamTierlistCache>>>,
}

/// A cheap "did the roster change" fingerprint — the operator data files' modified-time plus
/// their byte length, hashed. Used so the Team Tier List auto-regenerates the next time it's
/// requested after `data/Automated_Operators.json` (or `operators.json`) changes — e.g. after
/// new operators are added or the Operator Editor saves an edit — instead of silently serving a
/// stale cache forever until someone remembers to hit Recalculate.
fn operators_data_signature() -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for path in ["../data/Automated_Operators.json", "../data/operators.json"] {
        if let Ok(meta) = std::fs::metadata(path) {
            meta.len().hash(&mut hasher);
            if let Ok(modified) = meta.modified() {
                modified.hash(&mut hasher);
            }
        }
    }
    hasher.finish()
}

#[tokio::main]
async fn main() {
    let state = AppState {
        team_tierlist_cache: std::sync::Arc::new(tokio::sync::RwLock::new(None)),
    };

    let app = Router::new()
        .nest_service("/static", ServeDir::new("../static"))
        .route("/", get(read_root))
        .route("/compare", get(comparisons_view))
        .route("/editor", get(operator_editor))
        .route("/tierlist", get(tierlist_view))
        .route("/teams", get(teams_view))
        .route("/api/team_score", post(get_team_score))
        .route("/api/team_tierlist", get(get_team_tierlist))
        .route("/api/team_tierlist/recalculate", post(recalculate_team_tierlist))
        .route("/enemy_tierlist", get(enemy_tierlist_view))
        .route("/api/enemy_tierlist_data", get(get_enemy_tierlist_data))
        .route("/api/tierlist/export", get(export_tierlist_csv))
        .route("/api/tierlist/export_all", get(export_tierlist_csv_all))
        .route("/api/enemy_tierlist/export", get(export_enemy_tierlist_csv))
        .route("/api/enemy_tierlist/export_all", get(export_enemy_tierlist_csv_all))
        .route("/api/simulate_batch", post(run_simulation_batch))
        .route("/api/tierlist_data", get(get_tierlist_data))
        .route("/api/operators", get(get_operators))
        .route("/api/simulate", post(run_simulation))
        .route("/api/operators/save", post(save_operator))
        .route("/api/operators/{op_id}", delete(delete_operator))
        .route("/enemy_editor", get(enemy_editor_view))
        .route("/api/enemies", get(get_enemies))
        .route("/api/enemies/save", post(save_enemy))
        .route("/api/enemies/{id}", delete(delete_enemy))
        .with_state(state);

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
    let operators = loader.operators_raw.get("operators").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    // Dedupe by name (e.g. Amiya has 3 raw entries for her alternate combat forms, but is a
    // single roster slot / operator in-game and in the tier list's name-keyed lookups) so this
    // count matches what the tier list actually displays.
    let mut seen_names: std::collections::HashSet<String> = std::collections::HashSet::new();
    let operators: Vec<&Value> = operators.iter()
        .filter(|op| !core::data_loader::is_excluded_operator(op))
        .filter(|op| seen_names.insert(op.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string()))
        .collect();

    let total_operators = operators.len();
    let mut by_rarity_map: std::collections::HashMap<i64, i64> = std::collections::HashMap::new();
    let mut by_profession_map: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    for op in &operators {
        let r = op.get("rarity").and_then(|v| v.as_i64()).unwrap_or(0);
        *by_rarity_map.entry(r).or_insert(0) += 1;
        let p = op.get("profession").and_then(|v| v.as_str()).unwrap_or("").to_string();
        if !p.is_empty() { *by_profession_map.entry(p).or_insert(0) += 1; }
    }
    // Pre-order into fixed (label, file, count) rows so the template does plain iteration
    // instead of map lookups (JSON object keys are always strings, which would silently
    // mismatch an integer rarity index in the template).
    let rarity_count_6 = *by_rarity_map.get(&6).unwrap_or(&0);
    let by_rarity: Vec<(i64, i64)> = vec![6, 5, 4, 3, 2, 1].into_iter()
        .map(|r| (r, *by_rarity_map.get(&r).unwrap_or(&0)))
        .collect();
    let class_order = [
        ("PIONEER", "pioneer", "Vanguard"), ("WARRIOR", "warrior", "Guard"),
        ("TANK", "tank", "Defender"), ("SNIPER", "sniper", "Sniper"),
        ("CASTER", "caster", "Caster"), ("MEDIC", "medic", "Medic"),
        ("SUPPORT", "support", "Supporter"), ("SPECIAL", "special", "Specialist"),
    ];
    let by_profession: Vec<(&str, &str, i64)> = class_order.iter()
        .map(|(prof, file, label)| (*label, *file, *by_profession_map.get(*prof).unwrap_or(&0)))
        .collect();

    let enemies_count = std::fs::read_to_string("../data/Automated_Enemies.json")
        .ok()
        .and_then(|s| serde_json::from_str::<Vec<serde_json::Value>>(&s).ok())
        .map(|v| v.len())
        .unwrap_or(0);

    let mut env = minijinja::Environment::new();
    env.set_loader(minijinja::path_loader("../templates"));

    let html = env.get_template("home.html").unwrap().render(minijinja::context! {
        total_operators => total_operators,
        by_rarity => by_rarity,
        by_profession => by_profession,
        rarity_count_6 => rarity_count_6,
        enemies_count => enemies_count,
        version => "1"
    }).unwrap();
    Html(html)
}

async fn comparisons_view() -> impl IntoResponse {
    let loader = core::data_loader::DataLoader::new("../data");
    let mut seen_names: std::collections::HashSet<String> = std::collections::HashSet::new();
    let operators: Vec<Value> = loader.operators_raw.get("operators").and_then(|v| v.as_array()).cloned().unwrap_or_default()
        .into_iter()
        .filter(|op| !core::data_loader::is_excluded_operator(op))
        .filter(|op| seen_names.insert(op.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string()))
        .collect();

    let mut env = minijinja::Environment::new();
    env.set_loader(minijinja::path_loader("../templates"));

    let html = env.get_template("comparisons.html").unwrap().render(minijinja::context! {
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

/// CSV-escapes a field (wraps in quotes and doubles internal quotes if it contains a comma,
/// quote, or newline) — operator/skill/module names can contain commas or quotes.
fn csv_field(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn tierlist_csv_header(with_category: bool, metrics: &[(&str, &str, bool, bool)]) -> String {
    let base = "Rank,Tier,Operator,Skill,Module,Score,Phys DPS,Arts DPS,True DPS,Ele DPS,Wave TTC,Boss TTC,HPS,EHP Phys,Support Value";
    let mut header = if with_category { format!("Category,{}", base) } else { base.to_string() };
    for (key, _, _, _) in metrics {
        header.push_str(&format!(",Rank ({}),Tier ({})", key, key));
    }
    header.push('\n');
    header
}

fn tier_from_pct(pct: f64) -> &'static str {
    if pct <= 0.025 { "OP" }
    else if pct <= 0.11 { "S" }
    else if pct <= 0.27 { "A" }
    else if pct <= 0.49 { "B" }
    else if pct <= 0.71 { "C" }
    else if pct <= 0.86 { "D" }
    else if pct <= 0.95 { "E" }
    else { "F" }
}

/// (metric key, JSON field it ranks by, ascending?, gated?) — mirrors the Tier List page's own
/// "RANKING METRIC" dropdown exactly (17 options: `general` plus the other 16), including its two
/// quirks: `wave`/`boss` sort ascending (lower time-to-clear is better) and `dp` only ranks
/// operators who actually generate DP (gated = true), matching the page filtering non-generators
/// out of that view entirely rather than ranking them at the bottom.
const OPERATOR_RANK_METRICS: &[(&str, &str, bool, bool)] = &[
    ("general", "score", false, false),
    ("dp", "dp", false, true),
    ("wave", "wave_ttc", true, false),
    ("boss", "boss_ttc", true, false),
    ("phys_dmg", "phys_dmg", false, false),
    ("arts_dmg", "arts_dmg", false, false),
    ("ele_dmg", "elemental_dmg", false, false),
    ("healing", "heal", false, false),
    ("heal_phys", "heal_phys", false, false),
    ("heal_arts", "heal_arts", false, false),
    ("heal_ele", "heal_ele", false, false),
    ("buffs", "buffs", false, false),
    ("debuffs", "debuffs", false, false),
    ("utility", "utility", false, false),
    ("surv", "surv", false, false),
    ("phys_surv", "phys_surv", false, false),
    ("arts_surv", "arts_surv", false, false),
];

/// (metric key, JSON field, ascending?, gated?) — mirrors the Enemy Tier List's "RANK BY"
/// dropdown exactly (8 options). All 8 sort descending (bigger = scarier) with no gating.
const ENEMY_RANK_METRICS: &[(&str, &str, bool, bool)] = &[
    ("threat_score", "threat_score", false, false),
    ("hp", "hp", false, false),
    ("ehp", "ehp", false, false),
    ("dps", "dps", false, false),
    ("atk", "atk", false, false),
    ("def", "def", false, false),
    ("res", "res", false, false),
    ("weight", "weight", false, false),
];

/// Adds a `Rank_<key>`/`Tier_<key>` pair of columns to every row for each metric, ranking WITHIN
/// this same `rows` slice — i.e. within whichever single category it already represents. This is
/// the second axis of the cross-analysis (target category x ranking metric for operators,
/// threat class x rank-by for enemies): the "_all" exports already vary category per row-group;
/// this bakes in how each row ranks under every OTHER metric too, all in one file.
fn annotate_metric_ranks(rows: &mut [Value], metrics: &[(&str, &str, bool, bool)]) {
    for (key, field, ascending, gated) in metrics {
        let mut indices: Vec<usize> = (0..rows.len())
            .filter(|&i| !gated || rows[i].get(*field).and_then(|v| v.as_f64()).unwrap_or(0.0) > 0.0)
            .collect();
        indices.sort_by(|&a, &b| {
            let va = rows[a].get(*field).and_then(|v| v.as_f64()).unwrap_or(0.0);
            let vb = rows[b].get(*field).and_then(|v| v.as_f64()).unwrap_or(0.0);
            if *ascending { va.partial_cmp(&vb) } else { vb.partial_cmp(&va) }.unwrap_or(std::cmp::Ordering::Equal)
        });
        let n = indices.len().max(1);
        for (rank0, idx) in indices.into_iter().enumerate() {
            let pct = (rank0 + 1) as f64 / n as f64;
            if let Some(obj) = rows[idx].as_object_mut() {
                obj.insert(format!("Rank_{}", key), serde_json::json!(rank0 + 1));
                obj.insert(format!("Tier_{}", key), serde_json::json!(tier_from_pct(pct)));
            }
        }
    }
}

fn tierlist_csv_row(config: &Value, category: Option<&str>, metrics: &[(&str, &str, bool, bool)]) -> String {
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

    let mut base = format!("{},{},{},{},{},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2}",
        rank, tier, csv_field(op), csv_field(skill), csv_field(module), score, phys, arts, true_d, ele_d, wave, boss, heal, phys_surv, support);
    for (key, _, _, _) in metrics {
        let mrank = config.get(format!("Rank_{}", key)).and_then(|v| v.as_u64());
        let mtier = config.get(format!("Tier_{}", key)).and_then(|v| v.as_str()).unwrap_or("");
        match mrank {
            Some(r) => base.push_str(&format!(",{},{}", r, mtier)),
            None => base.push_str(",,"), // gated out of this metric (e.g. no DP generation)
        }
    }
    base.push('\n');
    match category {
        Some(cat) => format!("{},{}", cat, base),
        None => base,
    }
}

async fn export_tierlist_csv(Query(q): Query<TierlistDataQuery>) -> impl IntoResponse {
    let category = q.category.as_deref().unwrap_or("general");
    let loader = core::data_loader::DataLoader::new("../data");
    let (_general, detailed) = compute_tierlist_for_category(&loader, category, true);

    let mut csv = tierlist_csv_header(false, &[]);
    for config in &detailed {
        csv.push_str(&tierlist_csv_row(config, None, &[]));
    }

    (
        [(axum::http::header::CONTENT_DISPOSITION, "attachment; filename=\"arknights_tier_list.csv\""),
         (axum::http::header::CONTENT_TYPE, "text/csv")],
        csv,
    )
}

/// All 8 operator target categories × all 17 ranking metrics, in one CSV — the two axes the Tier
/// List page lets you view live (target category via the category selector, ranking metric via
/// the "RANKING METRIC" dropdown) are otherwise only ever visible one-at-a-time on screen. A
/// "Category" column plus a `Rank (metric)`/`Tier (metric)` pair per metric bakes both into every
/// row, so e.g. you can filter to Category=boss and directly compare a row's Rank(boss) against
/// its Rank(arts_dmg) to see the specialization/generalist tradeoff the app's own scoring rewards.
async fn export_tierlist_csv_all() -> impl IntoResponse {
    let loader = core::data_loader::DataLoader::new("../data");
    let categories = ["general", "normal", "elite", "boss", "ra", "is", "cc", "dp"];

    let mut csv = tierlist_csv_header(true, OPERATOR_RANK_METRICS);
    for category in categories {
        let (_general, mut detailed) = compute_tierlist_for_category(&loader, category, true);
        annotate_metric_ranks(&mut detailed, OPERATOR_RANK_METRICS);
        for config in &detailed {
            csv.push_str(&tierlist_csv_row(config, Some(category), OPERATOR_RANK_METRICS));
        }
    }

    (
        [(axum::http::header::CONTENT_DISPOSITION, "attachment; filename=\"arknights_tier_list_all_categories.csv\""),
         (axum::http::header::CONTENT_TYPE, "text/csv")],
        csv,
    )
}

fn enemy_csv_header(with_category: bool, metrics: &[(&str, &str, bool, bool)]) -> String {
    let base = "Rank,Tier,Name,Threat Class,Threat Score,EHP,DPS,HP,ATK,DEF,RES,Attack Interval";
    let mut header = if with_category { format!("Category,{}", base) } else { base.to_string() };
    for (key, _, _, _) in metrics {
        header.push_str(&format!(",Rank ({}),Tier ({})", key, key));
    }
    header.push('\n');
    header
}

fn enemy_csv_row(e: &Value, category: Option<&str>, metrics: &[(&str, &str, bool, bool)]) -> String {
    let rank = e.get("rank").and_then(|v| v.as_u64()).unwrap_or(0);
    let tier = e.get("tier").and_then(|v| v.as_str()).unwrap_or("");
    let name = e.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let tier_type = e.get("tier_type").and_then(|v| v.as_str()).unwrap_or("");
    let threat = e.get("threat_score").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let ehp = e.get("ehp").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let dps = e.get("dps").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let hp = e.get("hp").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let atk = e.get("atk").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let def = e.get("def").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let res = e.get("res").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let interval = e.get("attack_interval").and_then(|v| v.as_f64()).unwrap_or(0.0);

    let mut base = format!("{},{},{},{},{:.0},{:.0},{:.0},{:.0},{:.0},{:.0},{:.1},{:.2}",
        rank, tier, csv_field(name), tier_type, threat, ehp, dps, hp, atk, def, res, interval);
    for (key, _, _, _) in metrics {
        let mrank = e.get(format!("Rank_{}", key)).and_then(|v| v.as_u64());
        let mtier = e.get(format!("Tier_{}", key)).and_then(|v| v.as_str()).unwrap_or("");
        match mrank {
            Some(r) => base.push_str(&format!(",{},{}", r, mtier)),
            None => base.push_str(",,"),
        }
    }
    base.push('\n');
    match category {
        Some(cat) => format!("{},{}", cat, base),
        None => base,
    }
}

#[derive(Deserialize)]
struct EnemyExportQuery {
    category: Option<String>,
}

async fn export_enemy_tierlist_csv(Query(q): Query<EnemyExportQuery>) -> impl IntoResponse {
    let category = q.category.as_deref().unwrap_or("all");
    let (enemies, _resolved) = compute_enemy_tierlist(category);

    let mut csv = enemy_csv_header(false, &[]);
    for e in &enemies {
        csv.push_str(&enemy_csv_row(e, None, &[]));
    }

    (
        [(axum::http::header::CONTENT_DISPOSITION, "attachment; filename=\"arknights_enemy_tier_list.csv\""),
         (axum::http::header::CONTENT_TYPE, "text/csv")],
        csv,
    )
}

/// All 4 enemy threat classes × all 8 "RANK BY" metrics, in one CSV — "all" plus each sub-tier
/// (Boss/Elite/Normal are each separately re-ranked, percentiles relative to that subset, not the
/// whole roster), each carrying a `Rank (metric)`/`Tier (metric)` pair for every rank-by metric
/// the Enemy Tier List page offers, so both axes of the cross-analysis are in one file.
async fn export_enemy_tierlist_csv_all() -> impl IntoResponse {
    let categories = ["all", "boss", "elite", "normal"];
    let mut csv = enemy_csv_header(true, ENEMY_RANK_METRICS);
    for category in categories {
        let (mut enemies, _resolved) = compute_enemy_tierlist(category);
        annotate_metric_ranks(&mut enemies, ENEMY_RANK_METRICS);
        for e in &enemies {
            csv.push_str(&enemy_csv_row(e, Some(category), ENEMY_RANK_METRICS));
        }
    }

    (
        [(axum::http::header::CONTENT_DISPOSITION, "attachment; filename=\"arknights_enemy_tier_list_all_categories.csv\""),
         (axum::http::header::CONTENT_TYPE, "text/csv")],
        csv,
    )
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
                    op.change_state(Some(skill_index.to_string()));
                } else {
                    op.equipped_skill = None;
                    op.change_state(None);
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

/// Discounts a CC-duration stat (fear/frighten/stun/slow) by the trigger probability of the
/// SAME source skill/talent, when one exists (Lappland the Decadenza's S1: "浮游单元攻击时有
/// {prob}%几率使目标恐惧{fear}秒" — a per-hit chance, not a guaranteed proc on every hit).
/// calculate_stat() has no notion of this and would credit the full duration as unconditional.
/// Falls back to the plain duration (prob = 1.0) when no matching "prob" buff shares the name.
fn cc_duration_with_prob(op: &core::models::Operator, stat: &str) -> f64 {
    let buffs = op.get_active_buffs();
    let mut total = 0.0f64;
    for b in &buffs {
        if b.stat != stat { continue; }
        let dur = b.value.as_f64().unwrap_or(0.0);
        if dur <= 0.0 { continue; }
        let prob = buffs.iter()
            .find(|pb| pb.name == b.name && pb.stat == "prob")
            .and_then(|pb| pb.value.as_f64())
            .filter(|p| *p > 0.0 && *p < 1.0)
            .unwrap_or(1.0);
        total = f64::max(total, dur * prob);
    }
    total
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
        
        let is_sakiko_s2 = (dummy_op.name.contains("Sakiko") || dummy_op.is_char("char_4182_oblvns")) && skill_idx == Some(1);
        // Sakiko's S2 is a switch skill (Piano/Organ stances, same `equipped_skill` but two
        // distinct damage profiles) — run both stances as separate configs instead of one.
        let stance_variants: Vec<&str> = if is_sakiko_s2 { vec!["piano", "organ"] } else { vec![""] };

        for stance in &stance_variants {
        for mod_idx in modules_to_test.clone() {
            let mut op = dummy_op.clone();
            if let Some(si) = skill_idx {
                op.equipped_skill = Some(op.skills[si].clone());
                op.change_state(Some(si.to_string()));
            } else {
                op.change_state(None);
            }
            if !stance.is_empty() { op.skill_variant = stance.to_string(); }
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
            let is_boss_cat = category == "boss" || category == "cc";
            target_stats.insert("is_boss".to_string(), if is_boss_cat { 1.0 } else { 0.0 });
            
            let mut sim = core::simulation::SimulationEnvironment::new(op.clone(), None, Some(target_stats));
            
            let (_dmg_t, heal_t, _dp_t, _dmg_e, _heal_e, mut dmg_split) = sim.run_5_minute_sim();
            let (wave_ttc_raw, wave_ttc, wave_leaks) = sim.run_wave_sim(avg_enemy.hp, avg_enemy.def, avg_enemy.res);
            let (boss_hp, boss_def, boss_res) = if is_boss_cat {
                (avg_enemy.hp, avg_enemy.def, avg_enemy.res)
            } else {
                (80000.0, 1200.0, 50.0)
            };
            let (boss_ttc_raw, boss_ttc, boss_leak_ratio) = sim.run_boss_sim(boss_hp, boss_def, boss_res);
            
            if let Some(phys) = dmg_split.get_mut("physical") {
                *phys *= 1.0 - avg_enemy.dodge_phys;
            }
            if let Some(arts) = dmg_split.get_mut("arts") {
                *arts *= 1.0 - avg_enemy.dodge_arts;
            }

            let t_skill_time = *dmg_split.get("t_skill").unwrap_or(&300.0);
            if t_skill_time <= 0.0 {
                // If skill was never activated during combat (e.g. Duelist unable to block heavier enemy to charge SP),
                // evaluate survivability and field stats in unbuffed base state.
                op.change_state(None);
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
            // "寒冷" (Cold) — SilverAsh the Reignfrost's S2 signature mechanic (and anyone else's
            // kit using the same blackboard key): 2 stacks converts into a full Frozen lockdown
            // in-game, but this engine has no separate Frozen-stacking model, so at minimum the
            // Cold application itself (a real debuff on its own) wasn't credited anywhere at all.
            if op.calculate_stat("cold") > 0.0 { niche_score += 2.0; }
            if op.name == "Ines" {
                // Her talent "影织" roots (束缚) EVERY enemy for 5s the first time she damages
                // them — not a single-target lock. Across a full wave of enemies that's
                // functionally near-blanket AoE CC (a new bind on essentially every new target
                // she hits), not a one-off niche debuff. This engine has no generic "root
                // duration" stat (only stun/frighten/fear/slow/silence are tracked), so without
                // this her strongest kit element scored as literally nothing.
                niche_score += 3.0;
            }

            // A push/pull force below the target's weight does NOTHING in-game — the enemy just
            // doesn't move. `f64::max(0.5, force_weight)` used to floor this at a guaranteed
            // minimum credit even when `pp_force` was far below `avg_enemy.weight` (i.e. the
            // pull would visibly fail against a real target), crediting force/weight matchups
            // that flat-out don't work. Floored at 0.0 instead: no margin over the target's
            // weight means no credit. The overall scale is also cut (0.5 -> 0.25) since even a
            // genuinely successful push/pull is inherently map- and positioning-dependent (needs
            // open space behind/around the target) rather than a reliably-usable effect on every
            // stage, unlike a straightforward CC duration.
            let pp_targets = op.calculate_stat("push_pull_targets");
            let pp_force = op.calculate_stat("push_pull_force");
            if pp_targets > 0.0 {
                let force_margin = pp_force - avg_enemy.weight;
                niche_score += force_margin.max(0.0) * pp_targets * 0.25;
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
            // Buffs/Debuffs/Utility: same underlying numbers as support_score, split into the
            // three categories requested to replace the single catch-all "Support" tab, since
            // a pure buffer (team ATK/DEF/RES), a pure debuffer (enemy DEF/RES shred, fragile),
            // and a field-utility operator (DP, block, CC, camo, summon slot cost) don't compare
            // meaningfully against each other on one axis.
            let mut buff_score = 0.0;
            let mut debuff_score = 0.0;
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

            // Some kits mention "友方" only to state a PRESENCE/COUNT CONDITION that gates a
            // SELF buff ("当周围没有其他友方单位时，攻击力+8%" / "若有2个及以上友方单位，维娜
            // 技力回复速度+X" — Ceobe, Scavenger, Vina Victoria), not to grant the stat to
            // those allies. A bare `desc.contains("友方")` treats these as team-wide buffs.
            // Detect: every "友方" occurrence sits right after a condition marker (若/当/没有/
            // a digit or Chinese numeral counting units) or right before "时" closing the clause.
            let ally_mentions_are_condition_only = |desc: &str| -> bool {
                let mut idx = 0usize;
                let mut found_any = false;
                let mut all_conditional = true;
                while let Some(pos) = desc[idx..].find("友方") {
                    let abs = idx + pos;
                    found_any = true;
                    let before_tail: String = desc[..abs].chars().rev().take(10).collect::<Vec<_>>().into_iter().rev().collect();
                    let after_head: String = desc[abs..].chars().take(8).collect();
                    let is_conditional = before_tail.contains('若') || before_tail.contains('当') || before_tail.contains("没有")
                        || before_tail.chars().any(|c| c.is_ascii_digit() || "一二三四五六七八九十".contains(c))
                        || after_head.contains('时');
                    if !is_conditional { all_conditional = false; }
                    idx = abs + "友方".len();
                }
                found_any && all_conditional
            };

            // Check whether a buff realistically applies to allies based on stat & description
            let is_ally_buff = |st: &str, desc: &str, is_app: bool| -> bool {
                if is_app { return true; }
                if ally_mentions_are_condition_only(desc) { return false; }
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
                        || desc.contains("目标及自身") || desc.contains("周围友方") || desc.contains("其他友方") || desc.contains("治疗跳跃")
                    },
                    _ => false
                }
            };

            let mut provided_buffs = Vec::new();
            // Talents (values pass through the talent-modifier pipeline so that skills
            // like Mon3tr S2 `talent_scale` multiply the targeted talent's buffs)
            for t in &op.talents {
                for b in t.get_effective_buffs(Some(&op), None) {
                    if is_ally_buff(&b.stat, &t.description, t.applicable_to_others) {
                        let mut cb = b;
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
                let avg_sanc = f64::min(0.85, ((sanc_base * (1.0 - uptime)) + (sanc_base * sanc_scale * uptime)) * op.sanctuary_hp_gate_factor());
                let sanc_mit = avg_sanc * 1500.0 * 300.0;
                heal_phys_mitigation += sanc_mit;
                heal_arts_mitigation += sanc_mit;
                heal_ele_mitigation += sanc_mit;
                support_score += avg_sanc * 800.0;
                buff_score += avg_sanc * 800.0;
            }

            for (b, is_skill) in &provided_buffs {
                let raw_val = b.value.as_f64().unwrap_or(0.0).abs();
                let mult = if *is_skill { uptime } else { 1.0 };
                let val = raw_val * mult;
                
                match b.stat.as_str() {
                    "atk" => {
                        let add = if b.buff_type == "ratio" || ((b.buff_type == "blackboard" || b.buff_type.is_empty()) && raw_val <= 2.5) { val * 300.0 } else { val * 0.3 };
                        support_score += add; buff_score += add;
                    },
                    "aspd" | "attack_speed" => { support_score += val * 3.0; buff_score += val * 3.0; },
                    "hp" | "max_hp" => {
                        let add = if b.buff_type == "ratio" || ((b.buff_type == "blackboard" || b.buff_type.is_empty()) && raw_val <= 2.5) { val * 300.0 } else { val * 0.3 };
                        support_score += add; buff_score += add;
                    },
                    "def" => {
                        if b.buff_type == "ratio" || raw_val <= 2.5 {
                            let mit = val * 500.0 * 3.0 * 300.0 * 0.5;
                            heal_phys_mitigation += mit;
                            support_score += val * 250.0; buff_score += val * 250.0;
                        } else {
                            let mit = val * 3.0 * 300.0 * 0.8;
                            heal_phys_mitigation += mit;
                            support_score += val * 1.0; buff_score += val * 1.0;
                        }
                    },
                    "magic_resistance" | "res" => {
                        if b.buff_type == "ratio" || raw_val <= 2.5 {
                            let mit = (val * 15.0 * 0.01) * 1500.0 * 300.0;
                            heal_arts_mitigation += mit;
                            support_score += val * 300.0; buff_score += val * 300.0;
                        } else {
                            let mit = (val * 0.01) * 1500.0 * 300.0;
                            heal_arts_mitigation += mit;
                            support_score += val * 10.0; buff_score += val * 10.0;
                        }
                    },
                    "prob" | "arts_dodge" => {
                        let mit = val * 1500.0 * 300.0;
                        heal_arts_mitigation += mit;
                        support_score += val * 500.0; buff_score += val * 500.0;
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
                        support_score += eff_res * 600.0; buff_score += eff_res * 600.0;
                    },
                    "sp_recovery" => { support_score += val * 150.0; buff_score += val * 150.0; },
                    "target_limit" => support_score += val * 20.0,
                    "talent_multiplier" => support_score += val * 10.0,
                    "fragile" => { support_score += val * 500.0; debuff_score += val * 500.0; },
                    "arts_fragile" => { support_score += val * 400.0; debuff_score += val * 400.0; },
                    "elemental_fragile" => { support_score += val * 350.0; debuff_score += val * 350.0; },
                    "healing_received_bonus" => { support_score += val * 100.0; buff_score += val * 100.0; },
                    _ => {}
                }
            }

            if dummy_op.char_id.as_deref().unwrap_or("").contains("char_179_cgbird") || op.name == "Nightingale" {
                heal_arts_mitigation += 48000.0;
                support_score += 150.0; buff_score += 150.0;
            }

            let (flat_def, ratio_def) = op.target_def_debuffs();
            support_score += flat_def.abs() * 1.0; debuff_score += flat_def.abs() * 1.0;
            support_score += ratio_def.abs() * 1500.0; debuff_score += ratio_def.abs() * 1500.0;

            let (flat_res, ratio_res) = op.target_res_debuffs();
            support_score += flat_res.abs() * 20.0; debuff_score += flat_res.abs() * 20.0;
            support_score += ratio_res.abs() * 2000.0; debuff_score += ratio_res.abs() * 2000.0;

            support_score += op.fragile().abs() * 1500.0; debuff_score += op.fragile().abs() * 1500.0;
            support_score += op.arts_fragile().abs() * 1200.0; debuff_score += op.arts_fragile().abs() * 1200.0;
            support_score += op.elemental_fragile().abs() * 1100.0; debuff_score += op.elemental_fragile().abs() * 1100.0;

            let target_aspd_debuff = op.calculate_stat("target_aspd_debuff");
            if target_aspd_debuff > 0.0 { let add = target_aspd_debuff * 5.0; support_score += add; debuff_score += add; }

            // Frighten and Fear have no dedicated immunity stat in the enemy dataset, but both
            // are "hard CC" (the enemy stops acting) exactly like Stun, so `stun_immune_ratio` is
            // reused as the closest verified proxy for them too — this makes their contribution
            // to the Debuff score correctly shrink against tougher (Elite/Boss) enemy tiers
            // instead of staying constant regardless of target.
            if silence > 0.0 { let add = (silence / 5.0) * 30.0 * (1.0 - avg_enemy.silence_immune_ratio); support_score += add; debuff_score += add; }
            let stun_dur = cc_duration_with_prob(&op, "stun_duration");
            if stun_dur > 0.0 { let add = (stun_dur / 2.0) * 25.0 * (1.0 - avg_enemy.stun_immune_ratio); support_score += add; debuff_score += add; }
            let frighten_dur = cc_duration_with_prob(&op, "frighten_duration");
            if frighten_dur > 0.0 { let add = (frighten_dur / 2.0) * 30.0 * (1.0 - avg_enemy.stun_immune_ratio); support_score += add; debuff_score += add; }
            let fear_dur = cc_duration_with_prob(&op, "fear_duration");
            if fear_dur > 0.0 { let add = (fear_dur / 3.0) * 20.0 * (1.0 - avg_enemy.stun_immune_ratio); support_score += add; debuff_score += add; }
            let slow_dur = cc_duration_with_prob(&op, "slow_duration");
            if slow_dur > 0.0 { let add = (slow_dur / 3.0) * 20.0; support_score += add; debuff_score += add; }
            let cold_dur = op.calculate_stat("cold");
            if cold_dur > 0.0 { let add = (cold_dur / 3.0) * 15.0; support_score += add; debuff_score += add; }
            if op.name == "Ines" {
                // Her talent roots EVERY enemy for 5s on first hit — not a single-target lock,
                // so score it like a hard-CC duration applied broadly across a wave rather than
                // a one-off proc. See the niche_score comment above for the full explanation.
                let add = (5.0 / 2.0) * 25.0 * (1.0 - avg_enemy.stun_immune_ratio);
                support_score += add; debuff_score += add;
            }

            if op.is_skill_active && op.equipped_skill.as_ref().map(|s| s.is_global).unwrap_or(false) {
                support_score *= 2.0;
                buff_score *= 2.0;
                debuff_score *= 2.0;
            }
            
            let mut eff_score = 0.0;
            if let Some(s) = &op.equipped_skill {
                let s_cost = s.sp_cost;
                let s_dur = s.duration;
                // Two real false-positive patterns for a blanket "100% uptime" credit:
                // 1. A skill can say "持续时间无限" (unlimited duration) while actually being a
                //    self-terminating execute mode, not a sustainable toggle — e.g. Surtr's S3
                //    "黄昏" ("gradually loses HP... {duration}s until it reaches {hp_ratio}% max
                //    HP/sec", i.e. an HP-drain skill that runs until it kills her, escalating
                //    into a real cooldown on repeated deaths). Text says infinite; reality is a
                //    burst window with a death clock. Detected via the "逐渐流失生命" (gradually
                //    losing HP) phrase this class of skill consistently uses.
                // 2. `sp_type == "8"` (is_passive()'s catch-all) assumes ANY such skill is
                //    always-on, but some are "free deploy trigger, then forced auto-retreat with
                //    an EXTENDED redeploy time" (e.g. Nearl the Radiant Knight's "逐夜烁光") — the
                //    opposite of cooldown-free. Detected via "自动撤退" (auto-retreat) + a
                //    redeploy-time-extension term in the same skill's text.
                // 3. Executors (Texas the Omertosa, etc.) run entirely on `sp_type == "8"`
                //    skills too — is_passive() reads that as "always active", but an Executor's
                //    whole archetype is a kill-chain: the skill only keeps re-triggering while
                //    she's landing killing blows, and breaking that chain means real idle
                //    downtime before she can go again. Judge her uptime the same
                //    duration-vs-redeploy way the Team Builder's per-member override already
                //    does, instead of a blanket 100 — this was previously ONLY applied there, so
                //    the base tier list (and anything ranking off it, like Team Builder's swap
                //    candidate search) still saw every Executor at a false 100%.
                let is_exec = op.is_executor();
                let is_death_timer = s.description.contains("流失生命");
                let is_forced_retreat = s.description.contains("自动撤退") && s.description.contains("再部署时间");
                if is_exec && s_dur > 0.0 && op.final_redeployment_time() > 0.0 {
                    eff_score = (s_dur / (s_dur + op.final_redeployment_time())) * 100.0;
                } else if is_forced_retreat && op.final_redeployment_time() > 0.0 && s_dur > 0.0 {
                    eff_score = (s_dur / (s_dur + op.final_redeployment_time())) * 100.0;
                } else if is_death_timer && s_dur > 0.0 {
                    let uptime_r = s_dur / (s_dur + s_cost.max(1.0));
                    eff_score = uptime_r * 100.0;
                } else if s.is_infinite_or_toggle() || s.is_passive() {
                    eff_score = 100.0;
                } else if s_cost > 0.0 && s_dur > 0.0 {
                    let mut uptime_r = s_dur / (s_dur + s_cost);
                    if s.sp_type == "INCREASE_WHEN_ATTACK" || s.sp_type == "INCREASE_WHEN_TAKING_DAMAGE" {
                        uptime_r = s_dur / (s_dur + (s_cost * 2.5));
                    }
                    eff_score = uptime_r * 100.0;
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
            
            // heal_phys/arts/ele_mitigation are the SAME damage-resistance/dodge/sanctuary
            // value re-expressed three times, once per incoming-damage type (each is its own
            // display metric under heal_phys/heal_arts/heal_ele). Summing all three into the
            // aggregate score would triple-count one defensive effect; average them instead so
            // a Sanctuary/dodge buff counts once, at the same magnitude a mitigation-only
            // operator would show in any single channel.
            let mitigation_avg = (heal_phys_mitigation + heal_arts_mitigation + heal_ele_mitigation) / 3.0;
            let effective_heal = if is_team_healer {
                final_heal
                    + total_elemental_heal
                    + mitigation_avg
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
            if op.is_defender() && !op.is_duelist() {
                if field_block < 2.0 {
                    // 1-block Defender penalty: cannot hold multi-target lanes or weight >= 2 mob swarms.
                    // Duelists (Eunectes, etc.) are excluded: their 1-block is an intentional archetype
                    // trade-off for much higher personal stats, not a flaw to penalize like a mispositioned
                    // standard Defender.
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
            } else if category == "cc" {
                // Contingency Contract: hazard-buffed field means burst against the few
                // high-value threats and control to survive/lock them down both matter more
                // than in a standard run.
                total_score += (max_burst / 300.0) * 0.5 + niche_score * 25.0;
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

            let skill_tag = match skill_idx {
                None => "RAW".to_string(),
                Some(0) => "S1".to_string(),
                Some(1) => {
                    if is_sakiko_s2 {
                        if *stance == "organ" { "S2-2".to_string() } else { "S2-1".to_string() }
                    } else {
                        "S2".to_string()
                    }
                },
                Some(2) => "S3".to_string(),
                Some(i) => format!("S{}", i + 1),
            };

            // Utility: field-control/economy value that isn't a team buff or an enemy debuff —
            // DP generation, block/CC access (niche_score already covers camo, push/pull, status
            // resistance, CC durations). Kits built around a summon occupy an extra deployment
            // slot for the summon's lifetime — real opportunity cost the DPS/support numbers
            // don't capture — but no operator in this dataset has structured summon stats to
            // size that cost directly, so `has_summon_kit` is only recorded here (via kit text)
            // and the actual penalty is applied later against the population's average utility
            // score (see the percentile-scoring loop), the same way every other stat is judged
            // relative to the roster instead of a fixed constant.
            let has_summon_kit = dummy_op.talents.iter().any(|t| t.description.contains("召唤"))
                || dummy_op.skills.iter().any(|s| s.description.contains("召唤"));
            let utility_score = (niche_score * 50.0) + (total_dp_generated / 300.0 * 10.0);

            configs.push(serde_json::json!({
                "operator_name": op.name.clone(),
                "operator_id": dummy_op.operator_id.unwrap_or(0),
                "rarity": dummy_op.rarity,
                "rarity_bonus": match dummy_op.rarity { 6 => 20.0, 5 => 10.0, 4 => 5.0, _ => 0.0 },
                "photo_path": dummy_op.photo_path.clone(),
                "profession": dummy_op.profession.clone(),
                "subclass_name": dummy_op.subclass_name.clone(),
                "sub_profession_id": dummy_op.sub_profession_id.clone(),
                "position": dummy_op.position.clone(),
                "skill_name": op.equipped_skill.as_ref().map(|s| {
                    if is_sakiko_s2 {
                        format!("{} ({})", s.name, if *stance == "organ" { "Organ" } else { "Piano" })
                    } else {
                        s.name.clone()
                    }
                }).unwrap_or_else(|| "RAW".to_string()),
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
                "buffs": buff_score,
                "debuffs": debuff_score,
                "utility": utility_score,
                "has_summon_kit": has_summon_kit,
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
                "wave_ttc_raw": wave_ttc_raw,
                "wave_leaks": wave_leaks,
                "boss_ttc": boss_ttc,
                "boss_ttc_raw": boss_ttc_raw,
                "boss_leak_pct": boss_leak_ratio * 100.0,
            }));
        }
        } // stance_variants
    }
    
    Some(configs)
}
#[derive(serde::Deserialize)]
struct TierlistDataQuery {
    apply_decay: Option<bool>,
    category: Option<String>,
}

/// The full per-operator scoring/ranking pass (percentile-relative `score` + OP/S/A/B/C/D/E/F
/// `tier`) behind `/api/tierlist_data` for one category, extracted so it's callable outside the
/// HTTP handler too — `compute_team_tierlist_blocking` reuses the per-operator `score` this
/// produces to pick each class's top 5, instead of re-deriving "is this operator individually
/// good" from scratch.
fn compute_tierlist_for_category(loader: &core::data_loader::DataLoader, category: &str, apply_decay: bool) -> (Vec<Value>, Vec<Value>) {
    let target_enemy = core::enemy::get_enemy_by_category("../data", category);
    let raw_ops = loader.operators_raw.get("operators").and_then(|x| x.as_array());
    if raw_ops.is_none() {
        return (Vec::new(), Vec::new());
    }
    let raw_ops = raw_ops.unwrap();
    let mut all_configs: Vec<_> = raw_ops.par_iter().filter_map(|op_val| {
        if let Some(op_name) = op_val.get("name").and_then(|v| v.as_str()) {
            if core::data_loader::is_excluded_operator(op_val) { return None; }
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
        return (Vec::new(), Vec::new());
    }
    
    // "support" is intentionally excluded here: buff_score/debuff_score are its exact split
    // components, so keeping "support" in this list would score the same underlying value
    // twice (once whole, once split) in the composite score. The "support" JSON field itself
    // is kept for the CSV export / legacy readers, just not used for ranking anymore.
    let keys = vec!["niche", "buffs", "debuffs", "utility", "score_true_dmg", "score_elemental_dmg", "weakness_dmg", "ele_heal", "dp", "heal", "surv", "block", "score_arts_dmg", "score_phys_dmg", "potential_aoe", "efficiency", "max_burst"];
    
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
    
    // Arknights fields 8-12 operators out of a roster, not 1 — so a kit that maximizes a SINGLE
    // axis (Wiš'adel/Lemuen on armor shred, Lappland the Decadenza on pure arts DPS, Hoshiguma
    // the Breacher on survivability) earns its slot more reliably than one that's merely
    // "decent" across many axes, because the team can stack multiple specialists to cover what
    // a generalist would otherwise "cover a bit of everything". The old linear
    // `w_perf * (val/avg)` sum let exactly that kind of Swiss-army-knife operator (Skadi,
    // Eunectes) climb the tier list just by being modestly above average in MANY categories at
    // once, since those small edges added up the same as one category's real dominance. Raising
    // the ratio to a power > 1 fixes that without retuning any weight: at val==avg the ratio is
    // 1 and 1^k==1, so an average performance in a category scores exactly the same as before;
    // above-average ratios grow superlinearly (rewarding real specialization) while
    // below-average ratios shrink superlinearly (breadth of "fine, not great" stops being free).
    const SPECIALIZATION_EXPONENT: f64 = 1.5;

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
                } else if *key == "dp" {
                    // Outside the dedicated "DP Generators" category, DP generation still used
                    // the same default weight as any combat stat (w_perf 8) — badly undervaluing
                    // it relative to its real impact: Vanguards are close to a mandatory class in
                    // most game modes because a good one deploys the rest of the 12-operator
                    // roster almost immediately, a team-wide snowball effect this per-operator
                    // simulation has no way to measure directly. Weighted the same as the
                    // dedicated "DP Generators" category itself (36), so General no longer
                    // structurally buries the entire class behind damage/survivability stats it
                    // was never meant to compete on.
                    w_rarity = 4.0;
                    w_perf = 36.0;
                } else if *key == "heal" {
                    w_rarity = 4.0;
                    w_perf = 24.0;
                } else if *key == "support" {
                    w_rarity = 4.0;
                    w_perf = 16.0;
                } else if *key == "buffs" || *key == "debuffs" {
                    w_rarity = 4.0;
                    w_perf = 16.0;
                } else if *key == "utility" {
                    w_rarity = 3.0;
                    w_perf = 10.0;
                }
                
                let rarity_score = w_rarity * (1.0 - p);
                let ratio = (val / avg).max(0.0);
                let perf_score = w_perf * ratio.powf(SPECIALIZATION_EXPONENT);
                base_score += rarity_score + perf_score;
            }

            // Summon-kit slot cost: instead of a fixed constant, charge the same weighted
            // contribution an operator with exactly AVERAGE utility would have earned (w_perf
            // * 1.0, using the "utility" weight above) — the deployment slot the summon
            // occupies could otherwise have held an average-utility operator. This scales
            // automatically with the roster's actual utility average/weighting instead of a
            // magic number, so it stays correct as more operators are added.
            if obj.get("has_summon_kit").and_then(|v| v.as_bool()).unwrap_or(false) {
                let utility_w_perf = if category == "dp" { 3.0 } else { 10.0 };
                base_score -= utility_w_perf;
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
        
        let gen_entry = best.clone();
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
    
    (general_tierlist, all_configs)
}

async fn get_tierlist_data(Query(q): Query<TierlistDataQuery>) -> impl IntoResponse {
    let category = q.category.as_deref().unwrap_or("general");
    let loader = core::data_loader::DataLoader::new("../data");
    let apply_decay = q.apply_decay.unwrap_or(true);
    let target_enemy = core::enemy::get_enemy_by_category("../data", category);
    let (general_tierlist, all_configs) = compute_tierlist_for_category(&loader, category, apply_decay);
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

async fn teams_view() -> impl IntoResponse {
    let loader = core::data_loader::DataLoader::new("../data");
    let mut seen_names: std::collections::HashSet<String> = std::collections::HashSet::new();
    let operators: Vec<Value> = loader.operators_raw.get("operators").and_then(|v| v.as_array()).cloned().unwrap_or_default()
        .into_iter()
        .filter(|op| !core::data_loader::is_excluded_operator(op))
        .filter(|op| seen_names.insert(op.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string()))
        .collect();

    let mut env = minijinja::Environment::new();
    env.set_loader(minijinja::path_loader("../templates"));

    let html = env.get_template("teams.html").unwrap().render(minijinja::context! {
        operators => operators,
        version => "1"
    }).unwrap();
    Html(html)
}

#[derive(Deserialize)]
struct TeamMemberSpec {
    name: String,
    // `locked: false` (the default — matches a plain `{"name": "..."}`) means fully auto: pick
    // whichever skill/module combo maximizes `total_score`, same as before this was configurable.
    // `locked: true` pins the EXACT loadout: `skill_index`/`module_index` of `None` means "RAW"/
    // "No Module" specifically, not "auto" — a locked choice never falls back to guessing.
    #[serde(default)]
    skill_index: Option<usize>,
    #[serde(default)]
    module_index: Option<usize>,
    #[serde(default)]
    locked: bool,
}

#[derive(Deserialize)]
struct TeamScoreRequest {
    members: Vec<TeamMemberSpec>,
}

fn team_val(v: &Value, key: &str) -> f64 {
    v.get(key).and_then(|x| x.as_f64()).unwrap_or(0.0)
}

fn team_best_config(configs: &[Value]) -> Option<&Value> {
    configs.iter().max_by(|a, b| {
        team_val(a, "total_score")
            .partial_cmp(&team_val(b, "total_score"))
            .unwrap_or(std::cmp::Ordering::Equal)
    })
}

/// Finds the config matching a specific (skill_name, module_name) pair — used when the user has
/// explicitly locked in a skill/module in the Team Builder instead of letting `total_score` pick.
/// Falls back to the best-by-`total_score` config when no explicit choice was made (both `None`)
/// or the requested combo isn't found in this category's configs (shouldn't normally happen).
fn pick_config<'a>(configs: &'a [Value], skill_name: Option<&str>, module_name: Option<&str>) -> Option<&'a Value> {
    if skill_name.is_none() && module_name.is_none() {
        return team_best_config(configs);
    }
    configs.iter().find(|c| {
        skill_name.map(|s| c.get("skill_name").and_then(|v| v.as_str()) == Some(s)).unwrap_or(true)
            && module_name.map(|m| c.get("module_name").and_then(|v| v.as_str()) == Some(m)).unwrap_or(true)
    }).or_else(|| team_best_config(configs))
}

/// Builds the cached per-operator snapshot a team score is aggregated from. Runs
/// `evaluate_single_operator` 3 times (boss/normal/general categories) for a SINGLE operator —
/// cheap compared to the full-roster scan `/api/tierlist_data` does, since a team has at most 12
/// members. Lives here (not in `core::team`) because `core` is shared by several standalone
/// debug/test binaries that don't have `evaluate_single_operator` in their crate root.
///
/// `skill_index`/`module_index` (`None` = "RAW"/"No Module", `Some(i)` = that index in
/// `op.skills`/`op.modules`) let the Team Builder lock in a specific loadout instead of always
/// taking whichever config maximizes `total_score` — `total_score` has no notion of uptime/
/// cooldown at all, so for an operator like Pramanix (whose strongest S3 has a much worse
/// duration-to-cost ratio than her S2) the auto-picked config can systematically understate
/// Consistency for a real, deliberate loadout choice.
fn build_member_profile(
    data_dir: &str, loader: &core::data_loader::DataLoader, name: &str,
    skill_index: Option<usize>, module_index: Option<usize>, locked: bool,
) -> Option<core::team::TeamMemberProfile> {
    let op = loader.get_operator(name)?;
    // When locked, an unset index means "RAW"/"No Module" EXACTLY, not "auto" — only the
    // unlocked (both-`None`) case falls through to `pick_config`'s best-by-`total_score` pick.
    let skill_name: Option<&str> = locked.then(|| {
        skill_index.and_then(|i| op.skills.get(i)).map(|s| s.name.as_str()).unwrap_or("RAW")
    });
    let module_name: Option<&str> = locked.then(|| {
        module_index.and_then(|i| op.modules.get(i)).map(|m| m.name.as_str()).unwrap_or("No Module")
    });

    let boss_enemy = core::enemy::get_enemy_by_category(data_dir, "boss");
    let normal_enemy = core::enemy::get_enemy_by_category(data_dir, "normal");
    let general_enemy = core::enemy::get_enemy_by_category(data_dir, "general");

    let boss_configs = evaluate_single_operator(name, true, loader, &boss_enemy, "boss")?;
    let normal_configs = evaluate_single_operator(name, true, loader, &normal_enemy, "normal")?;
    let general_configs = evaluate_single_operator(name, true, loader, &general_enemy, "general")?;

    let boss_cfg = pick_config(&boss_configs, skill_name, module_name)?;
    let normal_cfg = pick_config(&normal_configs, skill_name, module_name)?;
    let general_cfg = pick_config(&general_configs, skill_name, module_name)?;

    // `total_score` (used to pick "best config" above) has no DP-generation term at all — it's
    // purely damage/heal/surv/support/niche/block/burst — so it never actually favors a
    // Vanguard's DP-generating skill over their damage skill. For the Deploy Time axis we
    // specifically want whichever config generates the MOST dp/sec, straight from the same
    // already-computed general-category configs — UNLESS the user explicitly locked in a
    // loadout, in which case respect that choice's actual DP output instead of hunting for a
    // different, unrequested skill/module combo.
    let dp_cfg = if locked {
        general_cfg
    } else {
        general_configs.iter().max_by(|a, b| {
            team_val(a, "dp_per_sec").partial_cmp(&team_val(b, "dp_per_sec")).unwrap_or(std::cmp::Ordering::Equal)
        }).unwrap_or(general_cfg)
    };

    let boss_dmg_score = team_val(boss_cfg, "score_phys_dmg")
        + team_val(boss_cfg, "score_arts_dmg")
        + team_val(boss_cfg, "score_true_dmg")
        + team_val(boss_cfg, "score_elemental_dmg");

    // Armor penetration: DEF/RES ignore ratio (0..1, direct % mitigation bypassed) plus a small
    // bonus for flat ignore values, scaled down since a flat number isn't directly comparable to
    // a ratio — this is a tunable approximation, not a literal in-game formula.
    let def_ignore = op.def_ignore_ratio().clamp(0.0, 0.95) + (op.def_ignore_flat() / 800.0).min(0.3);
    let res_ignore = op.res_ignore_ratio().clamp(0.0, 0.95) + (op.res_ignore_flat() / 80.0).min(0.3);
    let armor_pen = (def_ignore + res_ignore) / 2.0;

    // Consistency: for redeploy-based archetypes (Executors), uptime is better judged as
    // "active field time vs. redeploy cooldown" rather than "skill duration vs. SP cost" (which
    // `efficiency` already captures correctly for everyone else).
    let redeploy_time = op.final_redeployment_time();
    let is_exec = op.is_executor();
    let general_skill_name = general_cfg.get("skill_name").and_then(|v| v.as_str()).unwrap_or("RAW");
    let equipped_duration = op.skills.iter().find(|s| s.name == general_skill_name).map(|s| s.duration).unwrap_or(0.0);
    let efficiency = if is_exec && equipped_duration > 0.0 && redeploy_time > 0.0 {
        (equipped_duration / (equipped_duration + redeploy_time) * 100.0).min(100.0)
    } else {
        team_val(general_cfg, "efficiency")
    };

    Some(core::team::TeamMemberProfile {
        operator_name: op.name.clone(),
        profession: op.profession.clone(),
        sub_profession_id: op.sub_profession_id.clone(),
        rarity: op.rarity as i64,
        boss_ttc: team_val(boss_cfg, "boss_ttc"),
        boss_leak_pct: team_val(boss_cfg, "boss_leak_pct"),
        boss_dmg_score,
        armor_pen,
        wave_ttc: team_val(normal_cfg, "wave_ttc"),
        wave_leaks: team_val(normal_cfg, "wave_leaks"),
        block: team_val(normal_cfg, "block"),
        surv: team_val(general_cfg, "surv"),
        heal: team_val(general_cfg, "heal"),
        status_resist: op.calculate_stat("status_resistance"),
        dp_per_sec: team_val(dp_cfg, "dp_per_sec"),
        dp_cost: team_val(general_cfg, "dp_cost"),
        efficiency,
        buffs: team_val(general_cfg, "buffs"),
        debuffs: team_val(general_cfg, "debuffs"),
        utility_score: team_val(general_cfg, "utility"),
    })
}

/// For a team's single weakest axis, ranks every roster operator NOT already on the team by a
/// per-axis metric read straight off the general-category tier list (already-computed fields,
/// no extra simulation) and returns the top 3 names — candidates who are actually strong in
/// exactly the area the team is lacking, not just "good overall".
/// `profession_counts_after_removal` is the team's class distribution AFTER the swapped-out
/// member is gone — used to skip any candidate whose class is already at team.rs's own "no more
/// than 3 of one class" soft cap (`role_balance`/`role_balance_multiplier`). Without this, a
/// candidate can top the single axis being searched for and still tank the overall score on
/// arrival by tripping that penalty — recommending a fix that makes the team worse overall.
/// `excluded_names` are operators that shouldn't show up in a "recommended pick" at all —
/// currently anything sharing a name with an `is_excluded_operator` raw entry (special-mode-only
/// operators like Reclamation Algorithm exclusives, e.g. Tulip's RA-only 6★ record).
fn axis_swap_candidates(
    general_rows: &[Value], team_names: &std::collections::HashSet<String>, axis: &str,
    profession_counts_after_removal: &HashMap<String, i32>, excluded_names: &std::collections::HashSet<String>,
) -> Vec<String> {
    let mut candidates: Vec<(&Value, f64)> = general_rows.iter()
        .filter(|r| {
            let name = r.get("operator_name").and_then(|v| v.as_str()).unwrap_or("");
            if name.is_empty() || team_names.contains(name) || excluded_names.contains(name) { return false; }
            let prof = r.get("profession").and_then(|v| v.as_str()).unwrap_or("");
            if axis == "deploy_speed" && prof != "PIONEER" { return false; }
            profession_counts_after_removal.get(prof).copied().unwrap_or(0) < 3
        })
        .map(|r| {
            let metric = match axis {
                "boss_killing" => {
                    let dmg = team_val(r, "score_phys_dmg") + team_val(r, "score_arts_dmg") + team_val(r, "score_true_dmg") + team_val(r, "score_elemental_dmg");
                    dmg / team_val(r, "boss_ttc").max(1.0)
                }
                "lane_holding" => (team_val(r, "block") + 1.0) / team_val(r, "wave_ttc").max(1.0) * 300.0,
                "resistance" => team_val(r, "surv") + team_val(r, "heal") * 2.0,
                "utility" => team_val(r, "buffs") + team_val(r, "debuffs") + team_val(r, "utility"),
                "consistency" => team_val(r, "efficiency"),
                "deploy_speed" => team_val(r, "dp_per_sec"),
                _ => 0.0,
            };
            (r, metric)
        })
        .collect();
    candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    candidates.into_iter()
        .filter(|(_, m)| *m > 0.0001)
        .take(3)
        .filter_map(|(r, _)| r.get("operator_name").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .collect()
}

async fn get_team_score(Json(req): Json<TeamScoreRequest>) -> impl IntoResponse {
    let data_dir = "../data";
    let loader = core::data_loader::DataLoader::new(data_dir);
    let specs: Vec<&TeamMemberSpec> = req.members.iter().take(12).collect();

    let profiles: Vec<core::team::TeamMemberProfile> = specs.par_iter()
        .filter_map(|spec| build_member_profile(data_dir, &loader, &spec.name, spec.skill_index, spec.module_index, spec.locked))
        .collect();

    if profiles.is_empty() {
        return Json(serde_json::json!({ "error": "No valid operators in team." }));
    }

    let mut result = core::team::score_team(&profiles);

    // "Who should you swap out" — only meaningful once the team has enough members that dropping
    // one is a real tradeoff, not just "add more operators".
    if profiles.len() >= 4 {
        const ROLE_AXES: [&str; 5] = ["boss_killing", "lane_holding", "resistance", "utility", "consistency"];
        let axis_val = |name: &str| -> f64 { result["axes"][name].as_f64().unwrap_or(0.0) };
        let has_vanguard = profiles.iter().any(|p| p.profession == "PIONEER");
        // A missing Vanguard is a structural gap none of the 5 role axes fully capture (it's
        // only 20% of Utility, diluted further by averaging with 4 other axes) — the
        // "No Vanguard" recommendation already flags it, but the swap suggestion was still
        // chasing whichever role axis happened to be lowest, which could easily be unrelated
        // and never actually recommend a Vanguard at all. Prioritize fixing this first when it
        // applies, matching the same threshold (6+ members) as the recommendation/score penalty.
        let weak_axis = if !has_vanguard && profiles.len() >= 6 {
            Some("deploy_speed")
        } else {
            ROLE_AXES.iter().min_by(|a, b| {
                axis_val(a).partial_cmp(&axis_val(b)).unwrap_or(std::cmp::Ordering::Equal)
            }).copied()
        };

        if let (Some(axis), Some(weakest)) = (weak_axis, core::team::weakest_member(&profiles)) {
            let (general_tierlist, _) = compute_tierlist_for_category(&loader, "general", true);
            let team_names: std::collections::HashSet<String> = profiles.iter().map(|p| p.operator_name.clone()).collect();
            let mut profession_counts_after_removal: HashMap<String, i32> = HashMap::new();
            for p in &profiles {
                if p.operator_name == weakest { continue; }
                *profession_counts_after_removal.entry(p.profession.clone()).or_insert(0) += 1;
            }
            let mut excluded_names: std::collections::HashSet<String> = std::collections::HashSet::new();
            if let Some(raw_ops) = loader.operators_raw.get("operators").and_then(|v| v.as_array()) {
                for op_val in raw_ops {
                    if core::data_loader::is_excluded_operator(op_val) {
                        if let Some(n) = op_val.get("name").and_then(|v| v.as_str()) {
                            excluded_names.insert(n.to_string());
                        }
                    }
                }
            }
            let candidates = axis_swap_candidates(&general_tierlist, &team_names, axis, &profession_counts_after_removal, &excluded_names);
            if let Some(obj) = result.as_object_mut() {
                obj.insert("swap_recommendation".to_string(), serde_json::json!({
                    "remove": weakest,
                    "weak_axis": axis,
                    "add_candidates": candidates,
                }));
            }
        }
    }

    Json(result)
}

/// Builds a `TeamMemberProfile` for every non-excluded operator in the roster (used by the
/// auto-generated Team Tier List's genetic search) and runs the search. This is the expensive
/// path — roughly 3x the cost of one `/api/tierlist_data` full-roster scan, plus the GA itself —
/// so callers should run it inside `spawn_blocking` and cache the result.
/// How many of each class's individually-best operators are eligible for the auto-generated Team
/// Tier List's search pool. 8 classes × 5 = ~40 operators — a pool this size can't be searched
/// EXHAUSTIVELY (C(40,12) is still ~5.6 billion combinations), but it's small enough that the
/// genetic search covers it far more densely/thoroughly than the earlier ~214-operator B+-tier
/// pool did, at the cost of excluding good-but-not-top-5-in-class operators entirely (including
/// from the manual Team Builder is unaffected — this only gates the AUTO-generated list).
const TOP_N_PER_CLASS: usize = 5;

fn compute_team_tierlist_blocking() -> Vec<Value> {
    let data_dir = "../data";
    let loader = core::data_loader::DataLoader::new(data_dir);
    let (general_tierlist, _) = compute_tierlist_for_category(&loader, "general", true);

    let mut by_profession: HashMap<String, Vec<(String, f64)>> = HashMap::new();
    for c in &general_tierlist {
        let name = c.get("operator_name").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let profession = c.get("profession").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let score = c.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
        if name.is_empty() || profession.is_empty() { continue; }
        by_profession.entry(profession).or_default().push((name, score));
    }
    let mut names: Vec<String> = Vec::new();
    for entries in by_profession.values_mut() {
        entries.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        entries.truncate(TOP_N_PER_CLASS);
        names.extend(entries.iter().map(|(n, _)| n.clone()));
    }

    let profiles: HashMap<String, core::team::TeamMemberProfile> = names.par_iter()
        .filter_map(|name| build_member_profile(data_dir, &loader, name, None, None, false).map(|p| (name.clone(), p)))
        .collect();
    core::team::generate_team_tierlist(&profiles)
}

async fn get_team_tierlist(State(state): State<AppState>) -> impl IntoResponse {
    let current_signature = operators_data_signature();
    {
        let cache = state.team_tierlist_cache.read().await;
        if let Some(c) = cache.as_ref() {
            if c.data_signature == current_signature {
                return Json(serde_json::json!({ "status": "ready", "teams": c.teams }));
            }
            // The roster changed since this was computed (operator added/edited/removed) — fall
            // through and regenerate automatically instead of serving a stale list.
        }
    }
    let teams = tokio::task::spawn_blocking(compute_team_tierlist_blocking).await.unwrap_or_default();
    *state.team_tierlist_cache.write().await = Some(TeamTierlistCache { teams: teams.clone(), data_signature: current_signature });
    Json(serde_json::json!({ "status": "ready", "teams": teams }))
}

async fn recalculate_team_tierlist(State(state): State<AppState>) -> impl IntoResponse {
    let signature = operators_data_signature();
    let teams = tokio::task::spawn_blocking(compute_team_tierlist_blocking).await.unwrap_or_default();
    *state.team_tierlist_cache.write().await = Some(TeamTierlistCache { teams: teams.clone(), data_signature: signature });
    Json(serde_json::json!({ "status": "ready", "teams": teams }))
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
            op.change_state(Some(idx.to_string()));
        } else {
            op.change_state(None);
        }
    } else {
        op.change_state(None);
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

/// The full enrich/sort/rank pass behind `/api/enemy_tierlist_data`, extracted so the CSV export
/// endpoints can reuse it instead of re-deriving enemy rankings from scratch.
fn compute_enemy_tierlist(category_in: &str) -> (Vec<Value>, String) {
    let mut category = category_in.to_lowercase();
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

    // Some entries carry a NORMAL/ELITE/BOSS tier label copied from the game's own UI category,
    // but their stats actually belong to a special-mode encounter (CC hazard buffs, IS/RA
    // event-only enemies, etc.) rather than a real standard-campaign threat of that class
    // (e.g. `enemy_2093_skzams` is tagged NORMAL with ~500K HP). Drop those from the tier list
    // itself, not just the operator-scoring baseline, so they don't misrepresent that tier.
    enemies.retain(|e| {
        let t = e.get("tier_type").or_else(|| e.get("tier")).and_then(|v| v.as_str()).unwrap_or("").to_uppercase();
        let (hp_lo, hp_hi, def_lo, def_hi, res_lo, res_hi, atk_lo, atk_hi) = match t.as_str() {
            "NORMAL" => (1_000.0, 15_000.0, 100.0, 1_000.0, 10.0, 30.0, 100.0, 1_000.0),
            "ELITE" => (5_000.0, 30_000.0, 500.0, 2_500.0, 30.0, 60.0, 500.0, 2_000.0),
            "BOSS" => (15_000.0, 200_000.0, 1_000.0, 4_500.0, 45.0, 90.0, 2_000.0, 4_000.0),
            _ => return true,
        };
        let hp = e.get("hp").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let def = e.get("def").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let res = e.get("res").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let atk = e.get("atk").and_then(|v| v.as_f64()).unwrap_or(0.0);
        // ATK floor only applies when ATK is actually reported: many bosses/elites legitimately
        // deal 0 basic-attack damage (skill/aura-only kits), which this floor isn't meant to catch.
        let atk_below_floor = atk > 0.0 && atk < atk_lo;
        !(hp < hp_lo || hp > hp_hi || def < def_lo || def > def_hi || res < res_lo || res > res_hi || atk_below_floor || atk > atk_hi)
    });

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

    (enriched, category)
}

async fn get_enemy_tierlist_data(Query(params): Query<EnemyTierlistQuery>) -> impl IntoResponse {
    let category = params.category.unwrap_or_else(|| "all".to_string());
    let (enriched, category) = compute_enemy_tierlist(&category);
    let n = enriched.len();
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
                        let heal = c.get("raw_heal").and_then(|v| v.as_f64()).unwrap_or(0.0)
                            + c.get("ele_heal").and_then(|v| v.as_f64()).unwrap_or(0.0);
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
        for name in &["Haruka", "Eyjafjalla the Hvít Aska", "Mon3tr", "Nightingale"] {
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
        let mon3tr_h = max_heal_of("Mon3tr");
        let eyja_h = max_heal_of("Eyjafjalla the Hvít Aska");
        let nightingale_h = max_heal_of("Nightingale");

        // Pure healing-throughput hierarchy (raw + elemental restore, no mitigation credit):
        // Haruka's bubble HOT > Mon3tr's chain medic throughput > Hvít Aska > Nightingale (mitigation-focused)
        assert!(haruka_h > mon3tr_h, "Haruka ({}) should be > Mon3tr ({})", haruka_h, mon3tr_h);
        assert!(mon3tr_h > eyja_h, "Mon3tr ({}) should be > Eyjafjalla the Hvít Aska ({})", mon3tr_h, eyja_h);
        assert!(eyja_h > nightingale_h, "Eyjafjalla the Hvít Aska ({}) should be > Nightingale ({})", eyja_h, nightingale_h);
        assert!(nightingale_h > 100_000.0, "Nightingale should have strong multi-target healing");
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
                if let Some(configs) = evaluate_single_operator(op_name, true, &loader, &avg_enemy, "general") {
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
        for name in &["Ray", "Exusiai"] {
            for r in phys_results.iter().filter(|r| r.0 == *name) {
                println!("{} - Skill: {} => PHYS: {:.1}, SCORE_PHYS: {:.1}", r.0, r.1, r.5, r.6);
            }
        }

        println!("\n=== TARGET ARTS OPERATORS ===");
        for name in &["Lappland the Decadenza", "Pramanix the Prerita", "Logos"] {
            for r in arts_results.iter().filter(|r| r.0 == *name) {
                println!("{} - Skill: {} => ARTS: {:.1}, SCORE_ARTS: {:.1}", r.0, r.1, r.5, r.6);
            }
        }

        println!("\n=== TARGET ELEMENTAL OPERATORS ===");
        for name in &["Blaze the Igniting Spark", "Tragodia", "Mantra", "Logos"] {
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

        // Assert Physical hierarchy: Ray > Exusiai (base forms are calibrated below)
        let ray_p = max_phys_of("Ray");
        let exu_p = max_phys_of("Exusiai");
        assert!(ray_p > exu_p, "Ray ({}) must be > Exusiai ({})", ray_p, exu_p);

        // Assert Arts hierarchy: Pramanix the Prerita (sustained AoE formation caster) > Lappland the Decadenza > Logos
        let lapp_a = max_arts_of("Lappland the Decadenza");
        let pram_a = max_arts_of("Pramanix the Prerita");
        let logos_a = max_arts_of("Logos");
        assert!(pram_a > lapp_a, "Pramanix the Prerita ({}) must be > Lappland the Decadenza ({})", pram_a, lapp_a);
        assert!(lapp_a > logos_a, "Lappland the Decadenza ({}) must be > Logos ({})", lapp_a, logos_a);

        // Assert Elemental hierarchy: Blaze the Igniting Spark > Tragodia > Mantra > Logos
        let blaze_e = max_ele_of("Blaze the Igniting Spark");
        let trag_e = max_ele_of("Tragodia");
        let mantra_e = max_ele_of("Mantra");
        let logos_e = max_ele_of("Logos");
        assert!(blaze_e > trag_e, "Blaze the Igniting Spark ({}) must be > Tragodia ({})", blaze_e, trag_e);
        assert!(trag_e > mantra_e, "Tragodia ({}) must be > Mantra ({})", trag_e, mantra_e);
        assert!(mantra_e > logos_e, "Mantra ({}) must be > Logos ({})", mantra_e, logos_e);
    }
}


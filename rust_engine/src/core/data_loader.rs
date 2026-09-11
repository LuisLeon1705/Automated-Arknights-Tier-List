#![allow(dead_code)]
use super::models::{Operator, Buff, Talent};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Strips bracketed condition tags: "weak[limit]" -> "weaklimit"
fn remove_brackets(s: &str) -> String {
    let mut out = String::new();
    let mut depth = 0i32;
    for c in s.chars() {
        match c {
            '[' => depth += 1,
            ']' => { if depth > 0 { depth -= 1; } }
            _ => if depth == 0 { out.push(c); }
        }
    }
    out
}

/// Maps raw Arknights blackboard keys to the semantic stat names used by the engine.
/// Returns (new_stat, new_value, optional_explicit_buff_type).
/// `item_kind` is "talent", "skill" or "module" (negative atk/def on talents/modules
/// are enemy debuffs, while negative values on skills are usually self trade-offs).
/// `desc` is the source item's description, used to disambiguate the `prob` key
/// (trigger chance for crit/proc effects vs physical/arts dodge chance).
fn normalize_buff(raw: &str, val: f64, item_kind: &str, desc: &str) -> (String, f64, Option<String>) {
    let after_at = raw.rsplit('@').next().unwrap_or(raw);
    let has_dot = after_at.contains('.');
    let suffix0 = after_at.rsplit('.').next().unwrap_or(after_at);
    let suffix = remove_brackets(suffix0);
    let pct = |v: f64| -> f64 { if v.abs() >= 5.0 { v / 100.0 } else { v } };

    if suffix.starts_with("weak") {
        let nv = if val.abs() >= 1.0 { val / 100.0 } else { val };
        return if suffix.contains("magic") {
            ("arts_fragile".into(), nv, Some("ratio".into()))
        } else {
            ("fragile".into(), nv, Some("ratio".into()))
        };
    }

    match suffix.as_str() {
        "stun" => ("stun_duration".into(), val, Some("flat".into())),
        "sluggish" => ("slow_duration".into(), val, Some("flat".into())),
        "silence" => ("silence_duration".into(), val, Some("flat".into())),
        "fear" => ("fear_duration".into(), val, Some("flat".into())),
        "levitate" | "levitate_duration" | "buff_duration_levitate" => ("floating".into(), val, Some("flat".into())),
        "cam_duration" | "hidden_duration" => ("camouflage".into(), val, Some("flat".into())),
        "force" | "base_force_level" | "p_force" => ("push_pull_force".into(), val, Some("flat".into())),
        "block_cnt" => ("block_count".into(), val, Some("flat_base".into())),
        "respawn_time" => ("redeployment_time".into(), val, Some("flat_base".into())),
        "runtime_cost" => {
            if val.abs() < 50.0 { ("dp_cost".into(), val, Some("flat_base".into())) }
            else { ("ignored_runtime_cost".into(), val, Some("flat".into())) }
        },
        "def_penetrate" | "def_penetrate_ratio" => ("def_ignore_ratio".into(), pct(val), Some("ratio".into())),
        "def_penetrate_fixed" => ("def_ignore_flat".into(), val, Some("flat".into())),
        "magic_resist_penetrate_fixed" => ("res_ignore_flat".into(), val, Some("flat".into())),
        "magic_resist_penetrate_ratio" => ("res_ignore_ratio".into(), pct(val), Some("ratio".into())),
        "one_minus_status_resistance" => ("status_resistance".into(), -val, Some("flat".into())),
        "hp_recovery_per_sec_by_max_hp_ratio" => ("hp_regen".into(), pct(val), Some("ratio".into())),
        "hp_recovery_per_sec" => ("hp_regen".into(), val, Some("flat".into())),
        "damagenormal" | "damageranged" => ("flat_arts_dps".into(), val, Some("flat".into())),
        "times" => ("hits_abs".into(), val, Some("flat".into())),
        "max_target" => ("target_abs".into(), val, Some("flat".into())),
        "base_attack_time" => ("base_interval_flat".into(), val, Some("flat".into())),
        "sp" => ("instant_sp".into(), val, Some("flat".into())),
        "fake.b" => ("arts_fragile".into(), val, Some("ratio".into())),
        "move_speed" if has_dot && val < 0.0 => ("slow_duration".into(), val.abs(), Some("flat".into())),
        "damage_scale" if has_dot => ("fragile".into(), (val - 1.0).max(0.0), Some("ratio".into())),
        "damage_scale" if item_kind == "talent" && val > 1.0 && val < 2.0 => ("fragile".into(), val - 1.0, Some("ratio".into())),
        "magic_resistance" if val < 0.0 => ("target_res".into(), pct(val.abs()), Some("ratio".into())),
        "atk" if val < 0.0 && item_kind != "skill" => ("target_atk".into(), pct(val.abs()), Some("ratio".into())),
        "def" if val < 0.0 && item_kind != "skill" => ("target_def".into(), pct(val.abs()), Some("ratio".into())),
        "cost" if has_dot && (raw.contains("deck") || raw.contains("token") || raw.contains("hand")) => {
            ("hand_cost_reduce".into(), val, Some("flat".into()))
        },
        "cost" => {
            if val > 0.0 { ("dp_gain_per_cast".into(), val, Some("flat".into())) }
            else { ("ignored_cost".into(), val, Some("flat".into())) }
        },
        "attack_speed" => ("aspd".into(), val, Some("flat".into())),
        "talent_scale" => ("talent_multiplier".into(), val, Some("multiplier".into())),
        "ep_heal_ratio" => ("elemental_healing_per_second_atk_ratio".into(), val, Some("ratio".into())),
        "ep_damage_resistance" => ("ep_damage_resistance".into(), val, Some("ratio".into())),
        // "prob" is ambiguous in the raw blackboard: with dodge/block wording in the source
        // description it is a physical/arts dodge chance; otherwise it is the trigger
        // probability of a crit/proc effect (consumed by damage_multiplier).
        "prob" if desc.contains("闪避") || desc.contains("抵挡") || desc.contains("格挡") => ("arts_dodge".into(), val, Some("ratio".into())),
        "prob" => ("prob".into(), val, Some("ratio".into())),
        "magic_resistance" if val > 0.0 => {
            if val <= 2.0 { ("res".into(), val, Some("ratio".into())) }
            else { ("res".into(), val, Some("flat".into())) }
        },
        _ => {
            if raw.contains("ep_heal") {
                ("elemental_healing_per_second_atk_ratio".into(), val, Some("ratio".into()))
            } else if raw.contains("shield") && (raw.contains("ep") || raw.contains("atk_scale") || raw.contains("barrier")) {
                ("elemental_barrier".into(), val, Some("flat".into()))
            } else {
                (suffix, val, None)
            }
        },
    }
}

pub struct DataLoader {
    pub data_dir: String,
    pub classes: Value,
    pub operators_raw: Value,
}

impl DataLoader {
    pub fn new(data_dir: &str) -> Self {
        let classes =
            Self::load_json(data_dir, "classes.json").unwrap_or_else(|_| serde_json::json!({}));
        let mut operators_raw =
            Self::load_json(data_dir, "operators.json").unwrap_or_else(|_| serde_json::json!({"operators": []}));

        // Merge Automated_Operators.json if it exists
        let auto_res = Self::load_json(data_dir, "Automated_Operators.json")
            .or_else(|_| Self::load_json(".", "../Automated_Operators.json"))
            .or_else(|_| Self::load_json(".", "./Automated_Operators.json"));
        match auto_res {
            Ok(auto_ops) => {
                if let (Some(base_arr), Some(auto_arr)) = (
                    operators_raw.get_mut("operators").and_then(|v| v.as_array_mut()),
                    auto_ops.get("operators").and_then(|v| v.as_array())
                ) {
                    for op in auto_arr {
                        base_arr.push(op.clone());
                    }
                } else if auto_ops.get("operators").is_some() {
                    operators_raw = auto_ops;
                }
            },
            Err(e) => {
                println!("FAILED TO LOAD Automated_Operators.json: {:?}", e);
            }
        }

        DataLoader {
            data_dir: data_dir.to_string(),
            classes,
            operators_raw,
        }
    }

    fn load_json(data_dir: &str, filename: &str) -> Result<Value, Box<dyn std::error::Error>> {
        let path = Path::new(data_dir).join(filename);
        if path.exists() {
            let data = fs::read_to_string(path)?;
            let v: Value = serde_json::from_str(&data)?;
            Ok(v)
        } else {
            Ok(serde_json::json!({}))
        }
    }

    pub fn get_operator(&self, name: &str) -> Option<Operator> {
        if let Some(ops) = self
            .operators_raw
            .get("operators")
            .and_then(|v| v.as_array())
        {
            for op_data in ops {
                if let Some(op_name) = op_data.get("name").and_then(|v| v.as_str()) {
                    if op_name == name {
                        return Some(self.build_operator(op_data.clone(), false));
                    }
                }
            }
        }
        None
    }

    pub fn build_operator(&self, op_data: Value, is_summon: bool) -> Operator {
        let mut class_data = None;
        let mut branch_data = None;

        if !is_summon {
            if let Some(classes) = self.classes.get("classes").and_then(|v| v.as_array()) {
                let op_class_id = op_data.get("class_id").and_then(|v| v.as_i64());
                let op_branch_id = op_data.get("branch_id").and_then(|v| v.as_i64());

                for cls in classes {
                    if cls.get("class_id").and_then(|v| v.as_i64()) == op_class_id {
                        class_data = Some(cls.clone());
                        if let Some(branches) = cls.get("branches").and_then(|v| v.as_array()) {
                            for br in branches {
                                if br.get("branch_id").and_then(|v| v.as_i64()) == op_branch_id {
                                    branch_data = Some(br.clone());
                                }
                            }
                        }
                    }
                }
            }
        }
        let mut op: Operator = match serde_json::from_value(op_data.clone()) {
            Ok(v) => v,
            Err(_e) => {
                Operator {
                    name: op_data
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown")
                        .to_string(),
                    base_stats: HashMap::new(),
                    class_data: class_data.clone(),
                    branch_data: branch_data.clone(),
                    position: op_data
                        .get("position")
                        .and_then(|v| v.as_str())
                        .unwrap_or("ground")
                        .to_string(),
                    photo_path: op_data
                        .get("photo_path")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    is_summon,
                    talents: vec![],
                    skills: vec![],
                    modules: vec![],
                    summons: vec![],
                    ..Default::default()
                }
            }
        };

        op.class_data = class_data;
        op.branch_data = branch_data;
        op.is_summon = is_summon;
        if op.operator_id.is_none() {
            op.operator_id = op_data.get("operator_id").and_then(|v| v.as_i64());
        }
        if op.rarity == 0 {
            op.rarity = op_data.get("rarity").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        }

        if let Some(potentials) = op_data.get("potentials").and_then(|v| v.as_array()) {
            for p in potentials {
                if let Ok(buffs) = serde_json::from_value::<Vec<Buff>>(
                    p.get("buffs").cloned().unwrap_or(serde_json::json!([])),
                ) {
                    let level = p.get("level").and_then(|v| v.as_i64()).unwrap_or(1);
                    op.talents.push(Talent {
                        name: format!("Potential Lvl {}", level),
                        buffs,
                        ..Default::default()
                    });
                }
            }
        }

        if let Some(summons) = op_data.get("summons").and_then(|v| v.as_array()) {
            for s in summons {
                op.summons.push(self.build_operator(s.clone(), true));
            }
        }

        Self::normalize_operator(&mut op);

        op
    }

    /// Infers which talent a `talent_scale`/`talent_addition` buff targets by parsing
    /// the CN description ("第一天赋", "第二天赋", ...). Falls back to the first talent
    /// that has buffs when the text mentions "天赋" without an explicit index.
    fn infer_target_talent(desc: &str, talent_names: &[String], talent_has_buffs: &[bool]) -> Option<String> {
        let explicit = if desc.contains("第一天赋") || desc.contains("天赋一") || desc.contains("天赋①") {
            Some(0)
        } else if desc.contains("第二天赋") || desc.contains("天赋二") || desc.contains("天赋②") {
            Some(1)
        } else if desc.contains("第三天赋") || desc.contains("天赋三") || desc.contains("天赋③") {
            Some(2)
        } else {
            None
        };
        if let Some(i) = explicit {
            return talent_names.get(i).cloned().filter(|n| !n.is_empty());
        }
        if !desc.contains("天赋") {
            return None;
        }
        for (i, hb) in talent_has_buffs.iter().enumerate() {
            if *hb {
                return talent_names.get(i).cloned().filter(|n| !n.is_empty());
            }
        }
        None
    }

    fn link_talent_modifiers(op: &mut Operator) {
        let talent_names: Vec<String> = op.talents.iter().map(|t| t.name.clone()).collect();
        let talent_has_buffs: Vec<bool> = op.talents.iter()
            .map(|t| !t.buffs.is_empty() && !t.name.contains("Potential"))
            .collect();

        for s in &mut op.skills {
            let desc = s.description.clone();
            let needs = s.buffs.iter().chain(s.passive_buffs.iter())
                .any(|b| (b.stat == "talent_multiplier" || b.stat == "talent_addition") && b.target_talent.is_none());
            if needs {
                if let Some(name) = Self::infer_target_talent(&desc, &talent_names, &talent_has_buffs) {
                    for b in s.buffs.iter_mut().chain(s.passive_buffs.iter_mut()) {
                        if (b.stat == "talent_multiplier" || b.stat == "talent_addition") && b.target_talent.is_none() {
                            b.target_talent = Some(name.clone());
                        }
                    }
                }
            }
        }
        for m in &mut op.modules {
            let needs = m.buffs.iter()
                .any(|b| (b.stat == "talent_multiplier" || b.stat == "talent_addition") && b.target_talent.is_none());
            if needs {
                if let Some(name) = Self::infer_target_talent(&m.name, &talent_names, &talent_has_buffs) {
                    for b in m.buffs.iter_mut() {
                        if (b.stat == "talent_multiplier" || b.stat == "talent_addition") && b.target_talent.is_none() {
                            b.target_talent = Some(name.clone());
                        }
                    }
                }
            }
        }
    }

    /// Rewrites raw blackboard buff keys into the engine's semantic stat names.
    fn normalize_operator(op: &mut Operator) {
        fn apply_norm(b: &mut Buff, kind: &str, desc: &str) {
            let raw = b.stat.clone();
            let val = b.value.as_f64().unwrap_or(0.0);
            let (ns, nv, nt) = normalize_buff(&raw, val, kind, desc);
            b.stat = ns;
            if let Some(t) = nt { b.buff_type = t; }
            if b.value.as_f64().is_some() { b.value = serde_json::json!(nv); }
            b.current_decay_value = 0.0;
        }
        fn synth_pp(buffs: &mut Vec<Buff>) {
            let has_force = buffs.iter().any(|b| b.stat == "push_pull_force" && b.value.as_f64().unwrap_or(0.0) > 0.0);
            let has_targets = buffs.iter().any(|b| b.stat == "push_pull_targets");
            if has_force && !has_targets {
                buffs.push(Buff {
                    stat: "push_pull_targets".into(),
                    buff_type: "flat".into(),
                    value: serde_json::json!(1.0),
                    name: "Push/Pull".into(),
                    target_class: None,
                    target_position: None,
                    is_potential: false,
                    target_talent: None,
                    applies_to_summon: false,
                    is_global: false,
                    applies_to_allies: true,
                    applies_to_self: true,
                    immediate_dmg_kind: "physical".into(),
                    is_true_aoe: false,
                    decay: false,
                    decay_target_value: 0.0,
                    decay_time: 0.0,
                    current_decay_value: 0.0,
                });
            }
        }
        for t in &mut op.talents {
            let d = t.description.clone();
            for b in &mut t.buffs { apply_norm(b, "talent", &d); }
            synth_pp(&mut t.buffs);
        }
        for s in &mut op.skills {
            let d = s.description.clone();
            for b in &mut s.buffs { apply_norm(b, "skill", &d); }
            for b in &mut s.passive_buffs { apply_norm(b, "skill", &d); }
            for b in &mut s.overdrive_buffs { apply_norm(b, "skill", &d); }
            synth_pp(&mut s.buffs);
        }
        for m in &mut op.modules {
            for b in &mut m.buffs { apply_norm(b, "module", ""); }
            synth_pp(&mut m.buffs);
        }
        for s in &mut op.summons {
            Self::normalize_operator(s);
        }
        Self::link_talent_modifiers(op);
    }

    pub fn save_operators(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Path::new(&self.data_dir).join("operators.json");
        let data = serde_json::to_string_pretty(&self.operators_raw)?;
        fs::write(path, data)?;
        Ok(())
    }

    pub fn add_operator(&mut self, mut op_data: Value) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(obj) = self.operators_raw.as_object_mut() {
            if !obj.contains_key("operators") {
                obj.insert("operators".to_string(), serde_json::json!([]));
            }
        }

        if let Some(ops) = self
            .operators_raw
            .get_mut("operators")
            .and_then(|v| v.as_array_mut())
        {
            if op_data.get("operator_id").is_none() {
                let max_id = ops
                    .iter()
                    .filter_map(|op| op.get("operator_id").and_then(|id| id.as_i64()))
                    .max()
                    .unwrap_or(0);
                if let Some(obj) = op_data.as_object_mut() {
                    obj.insert("operator_id".to_string(), serde_json::json!(max_id + 1));
                }
            }
            ops.push(op_data);
        }
        self.save_operators()
    }

    pub fn update_operator(
        &mut self,
        op_id: i64,
        mut op_data: Value,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        if let Some(ops) = self
            .operators_raw
            .get_mut("operators")
            .and_then(|v| v.as_array_mut())
        {
            for op in ops.iter_mut() {
                if op.get("operator_id").and_then(|id| id.as_i64()) == Some(op_id) {
                    if let Some(obj) = op_data.as_object_mut() {
                        obj.insert("operator_id".to_string(), serde_json::json!(op_id));
                    }
                    *op = op_data;
                    self.save_operators()?;
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    pub fn delete_operator(&mut self, op_id: i64) -> Result<bool, Box<dyn std::error::Error>> {
        let mut modified = false;
        if let Some(ops) = self
            .operators_raw
            .get_mut("operators")
            .and_then(|v| v.as_array_mut())
        {
            let initial_len = ops.len();
            ops.retain(|op| op.get("operator_id").and_then(|id| id.as_i64()) != Some(op_id));
            if ops.len() < initial_len {
                modified = true;
            }
        }
        if modified {
            self.save_operators()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn get_all_operator_names(&self) -> Vec<String> {
        let mut names = Vec::new();
        if let Some(ops) = self
            .operators_raw
            .get("operators")
            .and_then(|v| v.as_array())
        {
            for op in ops {
                if let Some(name) = op.get("name").and_then(|v| v.as_str()) {
                    names.push(name.to_string());
                }
            }
        }
        names
    }
}

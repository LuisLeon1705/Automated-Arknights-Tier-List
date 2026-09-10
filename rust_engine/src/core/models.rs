#![allow(dead_code)]
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Deserializes a string field that might be null in JSON, returning "" for null.
fn nullable_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

/// Deserializes a field that may arrive as a JSON string OR number (e.g. sp_type "8" vs 8).
fn flexible_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let v = Value::deserialize(deserializer)?;
    Ok(match v {
        Value::String(s) => s,
        Value::Number(n) => n.to_string(),
        Value::Null => String::new(),
        other => other.to_string(),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Buff {
    #[serde(default, deserialize_with = "nullable_string")]
    pub stat: String,
    #[serde(alias = "type", default, deserialize_with = "nullable_string")]
    pub buff_type: String,
    #[serde(default)]
    pub value: Value,
    #[serde(default = "default_buff_name")]
    pub name: String,
    pub target_class: Option<String>,
    pub target_position: Option<String>,
    #[serde(default)]
    pub is_potential: bool,
    pub target_talent: Option<String>,
    #[serde(default)]
    pub applies_to_summon: bool,
    #[serde(default)]
    pub is_global: bool,
    #[serde(default)]
    pub applies_to_allies: bool,
    #[serde(default = "default_true")]
    pub applies_to_self: bool,
    #[serde(default = "default_dmg_kind")]
    pub immediate_dmg_kind: String,
    #[serde(default)]
    pub is_true_aoe: bool,
    #[serde(default)]
    pub decay: bool,
    #[serde(default)]
    pub decay_target_value: f64,
    #[serde(default)]
    pub decay_time: f64,

    // Dynamic sim fields
    #[serde(skip)]
    pub current_decay_value: f64,
}

fn default_buff_name() -> String {
    "Generic Buff".to_string()
}
fn default_true() -> bool {
    true
}
fn default_dmg_kind() -> String {
    "physical".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Summon {
    pub name: String,
    pub base_stats: HashMap<String, Value>,
    #[serde(default = "default_dmg_kind")]
    pub dmg_kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Talent {
    #[serde(default, deserialize_with = "nullable_string")]
    pub name: String,
    #[serde(default, deserialize_with = "nullable_string")]
    pub description: String,
    #[serde(default)]
    pub buffs: Vec<Buff>,
    #[serde(default)]
    pub applicable_to_others: bool,
    #[serde(default = "default_action_type")]
    pub action_type: String,
    #[serde(default = "default_mult")]
    pub dmg_mult: f64,
    #[serde(default)]
    pub heal_mult: f64,
    #[serde(default = "default_dmg_kind")]
    pub dmg_kind: String,
    #[serde(default)]
    pub is_global: bool,
    #[serde(default = "default_true")]
    pub applies_to_self: bool,
    #[serde(default)]
    pub is_true_aoe: bool,
    #[serde(default)]
    pub only_during_skill: bool,
    #[serde(default)]
    pub perm_camouflage: bool,
    #[serde(default)]
    pub summons_active: Vec<String>,
}

fn default_action_type() -> String {
    "none".to_string()
}
fn default_mult() -> f64 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Module {
    #[serde(default, deserialize_with = "nullable_string")]
    pub name: String,
    #[serde(default)]
    pub module_type: Option<String>,
    #[serde(default)]
    pub buffs: Vec<Buff>,
    #[serde(default)]
    pub is_global: bool,
    #[serde(default = "default_true")]
    pub applies_to_self: bool,
    #[serde(default)]
    pub is_true_aoe: bool,
    #[serde(default)]
    pub perm_camouflage: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Skill {
    #[serde(default, deserialize_with = "nullable_string")]
    pub name: String,
    #[serde(default, deserialize_with = "nullable_string")]
    pub description: String,
    #[serde(default)]
    pub sp_cost: f64,
    #[serde(default)]
    pub initial_sp: f64,
    #[serde(default)]
    pub duration: f64,
    #[serde(default)]
    pub buffs: Vec<Buff>,
    #[serde(default = "default_sp_type", deserialize_with = "flexible_string")]
    pub sp_type: String,
    #[serde(default)]
    pub applicable_to_others: bool,
    #[serde(default)]
    pub infinite_after: i32,
    #[serde(default = "default_skill_action_type")]
    pub action_type: String,
    #[serde(default = "default_mult")]
    pub dmg_mult: f64,
    #[serde(default)]
    pub heal_mult: f64,
    #[serde(default = "default_dmg_kind")]
    pub dmg_kind: String,
    #[serde(default)]
    pub passive_buffs: Vec<Buff>,
    #[serde(default)]
    pub is_global: bool,
    #[serde(default = "default_true")]
    pub applies_to_self: bool,
    #[serde(default)]
    pub is_true_aoe: bool,
    #[serde(default = "default_max_charges")]
    pub max_charges: i32,
    #[serde(default)]
    pub perm_camouflage: bool,
    #[serde(default)]
    pub is_ammo_based: bool,
    #[serde(default)]
    pub max_ammo: i32,
    #[serde(default = "default_ammo_cost")]
    pub ammo_cost: i32,
    #[serde(default)]
    pub summons_active: Vec<String>,
    #[serde(default)]
    pub despawn_summons_on_end: bool,
    #[serde(default)]
    pub limited_skill: bool,
    #[serde(default)]
    pub amount_skills: i32,
    #[serde(default)]
    pub limit_per_stage: bool,
    #[serde(default)]
    pub has_overdrive: bool,
    #[serde(default)]
    pub overdrive_buffs: Vec<Buff>,
    #[serde(default)]
    pub heal_on_activation_ratio: f64,

    // Dynamic sim fields
    #[serde(skip)]
    pub current_activations: i32,
    #[serde(skip)]
    pub is_active: bool,
    #[serde(skip)]
    pub times_used_in_stage: i32,
}

impl Skill {
    pub fn get_infinite_after(&self) -> i32 {
        if self.infinite_after > 0 {
            return self.infinite_after;
        }
        let desc = &self.description;
        let name = &self.name;
        let is_infinite = desc.contains("持续时间无限") 
            || desc.contains("Unlimited duration")
            || desc.contains("unlimited duration")
            || desc.contains("permanent")
            || desc.contains("切换")
            || desc.contains("toggle")
            || desc.contains("switches")
            || name == "Destreza"
            || name == "至高之术"
            || desc.contains("五次技能后");

        if is_infinite {
            if desc.contains("五次") || desc.contains("fifth") {
                5
            } else if desc.contains("四次") || desc.contains("fourth") {
                4
            } else if desc.contains("三次") || desc.contains("third") {
                3
            } else if desc.contains("第二次") || desc.contains("second") || name == "Destreza" || name == "至高之术" {
                2
            } else {
                1
            }
        } else {
            0
        }
    }

    pub fn is_passive(&self) -> bool {
        self.sp_type == "8"
            || self.sp_type == "Passive"
            || self.sp_type == "Inf-Passive"
            || (self.sp_cost <= 0.0 && self.duration <= 0.0)
    }

    pub fn is_infinite_or_toggle(&self) -> bool {
        self.is_passive() || self.get_infinite_after() > 0
    }
}

fn default_sp_type() -> String {
    "auto".to_string()
}
fn default_skill_action_type() -> String {
    "damage".to_string()
}
fn default_max_charges() -> i32 {
    1
}
fn default_ammo_cost() -> i32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Operator {
    pub name: String,
    #[serde(default)]
    pub operator_id: Option<i64>,
    #[serde(default)]
    pub char_id: Option<String>,
    #[serde(default)]
    pub base_stats: HashMap<String, Value>,
    pub class_data: Option<Value>,
    pub branch_data: Option<Value>,
    
    #[serde(default)]
    pub rarity: i32,
    #[serde(default)]
    pub profession: String,
    #[serde(default)]
    pub subclass_name: String,
    #[serde(default)]
    pub sub_profession_id: String,

    #[serde(default = "default_position")]
    pub position: String,
    #[serde(default)]
    pub photo_path: String,
    #[serde(default)]
    pub is_summon: bool,
    #[serde(default)]
    pub talents: Vec<Talent>,
    #[serde(default)]
    pub skills: Vec<Skill>,
    #[serde(default)]
    pub modules: Vec<Module>,
    #[serde(default)]
    pub summons: Vec<Operator>,

    #[serde(default)]
    pub always_active: bool,

    // Dynamic sim fields
    #[serde(skip)]
    pub owner_name: Option<String>,
    #[serde(skip)]
    pub sim_is_deployed: bool,
    #[serde(skip)]
    pub sim_redeploy_timer: f64,
    #[serde(skip)]
    pub sim_skill_timer: f64,
    #[serde(skip)]
    pub sim_attack_cooldown: f64,
    #[serde(skip)]
    pub sim_current_ammo: i32,
    #[serde(skip)]
    pub sim_immortality_timer: f64,
    #[serde(skip)]
    pub sim_stun_timer: f64,
    #[serde(skip)]
    pub sim_skill_uses_this_deployment: i32,
    #[serde(skip)]
    pub sim_hit_timer: f64,
    #[serde(skip)]
    pub sim_emergency_uses_count: i32,
    #[serde(skip)]
    pub sim_emergency_cooldown_timer: f64,
    #[serde(skip)]
    pub despawn_on_skill_end: bool,
    #[serde(skip)]
    pub original_skill: Option<Skill>,
    #[serde(skip)]
    pub equipped_skill: Option<Skill>,
    #[serde(skip)]
    pub current_hp: f64,
    #[serde(skip)]
    pub current_sp: f64,
    #[serde(skip)]
    pub external_buffs: Vec<Buff>,
    #[serde(skip)]
    pub is_skill_active: bool,
    #[serde(skip)]
    pub sim_drone_consecutive_attacks: i32,
    #[serde(skip)]
    pub sim_drone_cooldown: f64,
    #[serde(skip)]
    pub sim_immediate_sp_applied: bool,
    #[serde(skip)]
    pub sim_current_cast_time: f64,
    #[serde(skip)]
    pub active_skill: Option<Skill>,
    #[serde(skip)]
    pub id: usize, // useful for checking owner

    #[serde(skip)]
    pub current_state: String,
    
    #[serde(skip)]
    pub active_module: Option<Module>,
}

fn default_position() -> String {
    "ground".to_string()
}

pub type Entity = Operator;

#[derive(Debug, Clone)]
pub struct ImmediateInstance {
    pub mult: f64,
    pub kind: String,
    pub is_true_aoe: bool,
}

#[derive(Debug, Clone)]
pub struct HpLossInstance {
    pub value: f64,
    pub loss_type: String, // "ratio" or "flat"
}

#[derive(Debug, Clone)]
pub struct DpInstance {
    pub value: f64,
}

#[derive(Debug, Clone)]
pub struct HealInstance {
    pub value: f64,
    pub heal_type: String, // "ratio" or "flat"
    pub is_true_aoe: bool,
}

#[derive(Debug, Clone)]
pub struct ElementalBuildupInstance {
    pub target: String,
    pub amount: f64,
    pub amount_ratio: f64,
}

#[derive(Debug, Clone)]
pub struct AttackOverride {
    pub value: AttackOverrideValue,
}

#[derive(Debug, Clone)]
pub struct AttackOverrideValue {
    pub dmg_kind: String,
    pub atk_mod: f64,
    pub action_type: String, // "damage", "heal", "both"
    pub heal_mod: f64,
    pub dmg_ratio: f64,
    pub dmg_flat: f64,
    pub combo_attacks: Option<f64>,
}

impl Buff {
    pub fn applies_to(&self, op: &Operator) -> bool {
        if let Some(target_class) = &self.target_class {
            let op_class = op.class_data.as_ref().and_then(|c| c.get("name")).and_then(|n| n.as_str());
            if op_class != Some(target_class.as_str()) {
                return false;
            }
        }
        if let Some(target_position) = &self.target_position {
            if op.get_position() != *target_position {
                return false;
            }
        }
        true
    }
}

impl Talent {
    pub fn get_effective_buffs(&self, source_operator: Option<&Operator>, target_operator: Option<&Operator>) -> Vec<Buff> {
        if self.only_during_skill {
            if let Some(src) = source_operator {
                if !src.is_skill_active {
                    return vec![];
                }
            }
        }
        
        let mut dyn_add_flat = 0.0;
        let mut dyn_add_ratio = 0.0;
        let mut dyn_mult = 1.0;
        
        if let Some(src) = source_operator {
            for buff in src.get_talent_modifying_buffs() {
                let target = buff.target_talent.as_deref().unwrap_or("").trim().to_lowercase().replace(" ", "");
                let current = self.name.trim().to_lowercase().replace(" ", "");
                if target == current && !target.is_empty() {
                    if buff.stat == "talent_addition" {
                        if buff.buff_type == "ratio" {
                            dyn_add_ratio += buff.value.as_f64().unwrap_or(0.0);
                        } else {
                            dyn_add_flat += buff.value.as_f64().unwrap_or(0.0);
                        }
                    } else if buff.stat == "talent_multiplier" {
                        if buff.buff_type == "multiplier" {
                            dyn_mult *= buff.value.as_f64().unwrap_or(1.0);
                        } else {
                            dyn_mult += buff.value.as_f64().unwrap_or(0.0);
                        }
                    }
                }
            }
        }
        
        let mut effective = Vec::new();
        for buff in &self.buffs {
            if let Some(target) = target_operator {
                if !buff.applies_to(target) {
                    continue;
                }
            }
            let mut final_buff = buff.clone();
            if let Some(val) = buff.value.as_f64() {
                let final_val = (val + dyn_add_flat) * (1.0 + dyn_add_ratio) * dyn_mult;
                final_buff.value = serde_json::json!(final_val);
                final_buff.current_decay_value = final_val;
            }
            final_buff.name = self.name.clone();
            final_buff.is_global = buff.is_global || self.is_global;
            final_buff.applies_to_allies = buff.applies_to_allies || self.applicable_to_others;
            final_buff.applies_to_self = buff.applies_to_self && self.applies_to_self;
            final_buff.is_true_aoe = buff.is_true_aoe || self.is_true_aoe;
            effective.push(final_buff);
        }
        
        effective
    }
}

impl Operator {
    pub fn get_talent_modifying_buffs(&self) -> Vec<Buff> {
        let mut mods = Vec::new();
        if let Some(skill) = &self.equipped_skill {
            mods.extend(skill.passive_buffs.clone());
            if self.is_skill_active {
                mods.extend(skill.buffs.clone());
            }
        }
        if let Some(module) = &self.active_module {
            mods.extend(module.buffs.clone());
        }
        for t in &self.talents {
            if t.name.contains("Potential") {
                mods.extend(t.buffs.clone());
            }
        }
        mods
    }

    pub fn get_active_buffs(&self) -> Vec<Buff> {
        let mut buffs = Vec::new();
        for t in &self.talents {
            if t.name.contains("Potential") {
                continue;
            }
            buffs.extend(t.get_effective_buffs(Some(self), Some(self)));
        }
        for t in &self.talents {
            if t.name.contains("Potential") {
                for b in &t.buffs {
                    if b.applies_to(self) {
                        buffs.push(b.clone());
                    }
                }
            }
        }
        if let Some(skill) = &self.equipped_skill {
            for b in &skill.passive_buffs {
                if b.applies_to(self) {
                    let mut b_clone = b.clone();
                    b_clone.name = skill.name.clone();
                    b_clone.applies_to_allies = b.applies_to_allies || skill.applicable_to_others;
                    buffs.push(b_clone);
                }
            }
            if self.is_skill_active || skill.is_passive() {
                for b in &skill.buffs {
                    if b.applies_to(self) {
                        let mut b_clone = b.clone();
                        b_clone.name = skill.name.clone();
                        b_clone.applies_to_allies = b.applies_to_allies || skill.applicable_to_others;
                        buffs.push(b_clone);
                    }
                }
                if skill.has_overdrive && self.sim_skill_timer <= (skill.duration / 2.0) {
                    for b in &skill.overdrive_buffs {
                        if b.applies_to(self) {
                            let mut b_clone = b.clone();
                            b_clone.name = skill.name.clone();
                            b_clone.applies_to_allies = b.applies_to_allies || skill.applicable_to_others;
                            buffs.push(b_clone);
                        }
                    }
                }
            }
        }
        if let Some(module) = &self.active_module {
            for b in &module.buffs {
                if b.applies_to(self) {
                    let mut b_clone = b.clone();
                    b_clone.name = module.name.clone();
                    buffs.push(b_clone);
                }
            }
        }
        buffs.extend(self.external_buffs.clone());
        
        buffs.into_iter().filter(|b| {
            if self.is_summon {
                b.applies_to_summon
            } else {
                b.applies_to_self
            }
        }).collect()
    }

    pub fn calculate_stat(&self, name: &str) -> f64 {
        let mut base = 0.0;
        if let Some(b) = self.base_stats.get(name) {
            if let Some(val) = b.as_f64() {
                base = val;
            } else if let Some(arr) = b.as_array() {
                if let Some(last) = arr.last() {
                    base = last.as_f64().unwrap_or(0.0);
                }
            }
        }
        if let Some(trust_stats) = self.base_stats.get("trust_stats") {
            if let Some(val) = trust_stats.get(name).and_then(|v| v.as_f64()) {
                base += val;
            }
        }
        
        let meta_stats = ["hp", "atk", "def", "res", "aspd", "base_interval", "dp_cost", "redeployment_time", "block_count"];
        let is_meta = !meta_stats.contains(&name);
        
        let active = self.get_active_buffs();
        let mut r_sum = 0.0;
        let mut fb_sum = 0.0;
        let mut fi_sum = 0.0;
        let mut m_prod = 1.0;
        
        let mut check_names = vec![name.to_string()];
        if name == "hp" { check_names.push("max_hp".to_string()); check_names.push("hp_ratio".to_string()); }
        if name == "aspd" { check_names.push("attack_speed".to_string()); }
        if name == "res" { check_names.push("magic_resistance".to_string()); }
        if name == "base_interval" { check_names.push("base_attack_time".to_string()); }

        for b in active {
            if check_names.contains(&b.stat) {
                let mut val = b.current_decay_value;
                if val == 0.0 { 
                    val = b.value.as_f64().unwrap_or(0.0);
                }
                
                let b_type = if b.buff_type == "blackboard" || b.buff_type.is_empty() {
                    match b.stat.as_str() {
                        "atk" | "def" | "hp" | "max_hp" | "hp_ratio" | "atk_scale" | "def_scale" => "ratio",
                        "attack_speed" | "aspd" | "magic_resistance" | "res" => "flat",
                        _ => "flat"
                    }
                } else {
                    b.buff_type.as_str()
                };

                let mut as_flat = false;
                // CN format quirk: some percentage buffs are stored as 30.0 instead of 0.3.
                // Values >= 1000 are always flat additions (e.g. "Max HP +5000"), never ratios.
                if b_type == "ratio" {
                    if val.abs() >= 1000.0 {
                        as_flat = true;
                    } else if val >= 5.0 {
                        val /= 100.0;
                    }
                }

                if is_meta {
                    fb_sum += val;
                } else if as_flat {
                    fb_sum += val;
                } else {
                    if b_type == "ratio" {
                        r_sum += val;
                    } else if b_type == "flat" || b_type == "flat_base" {
                        fb_sum += val;
                    } else if b_type == "flat_insp" {
                        fi_sum += val;
                    } else if b_type == "multiplier" {
                        m_prod *= val;
                    }
                }
            }
        }
        
        if is_meta {
            return fb_sum;
        }
        
        ((base + fb_sum) * (1.0 + r_sum) + fi_sum) * m_prod
    }

    pub fn final_atk(&self) -> f64 {
        self.calculate_stat("atk")
    }

    pub fn final_def(&self) -> f64 {
        self.calculate_stat("def")
    }

    pub fn final_hp(&self) -> f64 {
        self.calculate_stat("hp")
    }

    pub fn final_interval(&self) -> f64 {
        let mut base = self.base_stats.get("base_interval").and_then(|v| v.as_f64()).unwrap_or(1.0);
        for b in self.get_active_buffs() {
            if b.stat == "base_interval_fixed" {
                base = b.value.as_f64().unwrap_or(base);
                break;
            }
        }
        
        let aspd = 100.0 + self.calculate_stat("aspd");
        
        let interval_mult = self.calculate_stat("interval_mult");
        let mult = if interval_mult > 0.0 { interval_mult } else { 1.0 };
        
        ((base + self.calculate_stat("base_interval_flat")) / (aspd / 100.0)) * mult
    }

    pub fn fragile(&self) -> f64 {
        let mut v = 0.0;
        for b in self.get_active_buffs() {
            if b.stat == "fragile" {
                let val = b.value.as_f64().unwrap_or(0.0);
                if val > v { v = val; }
            }
        }
        v
    }
    
    pub fn arts_fragile(&self) -> f64 {
        let mut v = 0.0;
        for b in self.get_active_buffs() {
            if b.stat == "arts_fragile" {
                let val = b.value.as_f64().unwrap_or(0.0);
                if val > v { v = val; }
            }
        }
        v
    }
    
    pub fn elemental_fragile(&self) -> f64 {
        let mut v = 0.0;
        for b in self.get_active_buffs() {
            if b.stat == "elemental_fragile" {
                let val = b.value.as_f64().unwrap_or(0.0);
                if val > v { v = val; }
            }
        }
        v
    }
    
    pub fn target_def_debuffs(&self) -> (f64, f64) {
        let mut f = 0.0;
        let mut r = 0.0;
        for b in self.get_active_buffs() {
            if b.stat == "target_def" {
                let val = b.value.as_f64().unwrap_or(0.0);
                if b.buff_type == "flat" || b.buff_type == "flat_base" {
                    f += val;
                } else if b.buff_type == "ratio" {
                    r += val;
                }
            }
        }
        (f, r)
    }
    
    pub fn target_res_debuffs(&self) -> (f64, f64) {
        let mut f = 0.0;
        let mut r = 0.0;
        for b in self.get_active_buffs() {
            if b.stat == "target_res" {
                let val = b.value.as_f64().unwrap_or(0.0);
                if b.buff_type == "flat" || b.buff_type == "flat_base" {
                    f += val;
                } else if b.buff_type == "ratio" {
                    r += val;
                }
            }
        }
        (f, r)
    }
    
    pub fn is_duelist(&self) -> bool {
        self.sub_profession_id == "duelist" 
            || self.subclass_name.to_lowercase().contains("duelist") 
            || self.subclass_name.contains("决战者")
    }

    pub fn is_char(&self, id: &str) -> bool {
        self.char_id.as_deref() == Some(id) || self.photo_path.contains(id)
    }

    pub fn def_ignore_flat(&self) -> f64 { self.calculate_stat("def_ignore_flat") }
    pub fn def_ignore_ratio(&self) -> f64 {
        let mut r = self.calculate_stat("def_ignore_ratio");
        for t in &self.talents {
            let mut pen = 0.0;
            let mut cnt = 1.0;
            for b in &t.buffs {
                if b.stat == "def_penetrate_ratio" || b.stat == "def_ignore_ratio" {
                    pen = b.value.as_f64().unwrap_or(0.0);
                } else if b.stat == "max_cnt" {
                    cnt = b.value.as_f64().unwrap_or(1.0);
                }
            }
            if pen > 0.0 {
                r = r.max(pen * cnt);
            }
        }
        r
    }
    pub fn res_ignore_flat(&self) -> f64 {
        let mut r = self.calculate_stat("res_ignore_flat");
        for t in &self.talents {
            let mut pen = 0.0;
            let mut cnt = 1.0;
            for b in &t.buffs {
                if b.stat.contains("magic_resist_penetrate_fixed") || b.stat.contains("res_penetrate_fixed") || b.stat.contains("res_ignore_flat") {
                    pen = b.value.as_f64().unwrap_or(0.0);
                } else if b.stat.contains("max_trigger_cnt") || b.stat == "max_cnt" || b.stat == "cnt" {
                    cnt = b.value.as_f64().unwrap_or(1.0);
                }
            }
            if pen > 0.0 {
                r = r.max(pen * cnt);
            }
        }
        for b in self.get_active_buffs() {
            if b.stat.contains("magic_resist_penetrate_fixed") || b.stat.contains("res_penetrate_fixed") || b.stat.contains("res_ignore_flat") {
                let pen = b.value.as_f64().unwrap_or(0.0);
                r = r.max(pen);
            }
        }
        r
    }
    pub fn res_ignore_ratio(&self) -> f64 {
        let mut r = self.calculate_stat("res_ignore_ratio");
        for t in &self.talents {
            let mut pen = 0.0;
            let mut cnt = 1.0;
            for b in &t.buffs {
                if b.stat == "magic_resist_penetrate_ratio" || b.stat == "res_penetrate_ratio" || b.stat == "res_ignore_ratio" {
                    pen = b.value.as_f64().unwrap_or(0.0);
                } else if b.stat == "max_cnt" {
                    cnt = b.value.as_f64().unwrap_or(1.0);
                }
            }
            if pen > 0.0 {
                r = r.max(pen * cnt);
            }
        }
        r
    }
    
    pub fn hp_regen_per_second(&self) -> f64 {
        let max_hp = self.final_hp();
        let mut r_sum = 0.0;
        let mut f_sum = 0.0;
        
        for b in self.get_active_buffs() {
            if b.stat == "hp_regen" {
                let mut val = b.current_decay_value;
                if val == 0.0 { val = b.value.as_f64().unwrap_or(0.0); }
                if b.buff_type == "ratio" {
                    r_sum += val;
                } else {
                    f_sum += val;
                }
            } else if b.stat == "damage_recovery" {
                if let Some(val) = b.value.as_f64() {
                    f_sum += val / 1.5;
                }
            }
        }
        
        (max_hp * r_sum) + f_sum
    }

    pub fn get_provided_buffs(&self, target: &Operator) -> Vec<Buff> {
        let mut prov = Vec::new();
        let is_my_summon = target.is_summon && target.owner_name.as_deref() == Some(&self.name);
        
        for t in &self.talents {
            if t.applicable_to_others || is_my_summon {
                prov.extend(t.get_effective_buffs(Some(self), Some(target)));
            }
        }
        if self.is_skill_active {
            if let Some(skill) = &self.equipped_skill {
                if skill.applicable_to_others || is_my_summon {
                    for b in &skill.buffs {
                        if b.applies_to(target) {
                            prov.push(b.clone());
                        }
                    }
                    if skill.has_overdrive && self.sim_skill_timer <= (skill.duration / 2.0) {
                        for b in &skill.overdrive_buffs {
                            if b.applies_to(target) {
                                prov.push(b.clone());
                            }
                        }
                    }
                }
            }
        }
        if let Some(module) = &self.active_module {
            for b in &module.buffs {
                if b.is_global || is_my_summon {
                    if b.applies_to(target) {
                        prov.push(b.clone());
                    }
                }
            }
        }
        
        let mut final_prov = Vec::new();
        for b in prov {
            if is_my_summon {
                if b.applies_to_summon { final_prov.push(b); }
            } else {
                if b.applies_to_allies { final_prov.push(b); }
            }
        }
        
        final_prov
    }

    pub fn change_state(&mut self, state: Option<String>) {
        if state.is_none() {
            self.is_skill_active = false;
            self.current_state = "No Skill".to_string();
        } else {
            self.is_skill_active = true;
            self.current_state = state.unwrap();
        }
    }
    
    pub fn reset_buff_decay(&mut self) {}

    pub fn emergency_hp_restore_threshold(&self) -> f64 { self.calculate_stat("emergency_hp_restore_threshold") }
    pub fn emergency_hp_restore_amount(&self) -> f64 { self.calculate_stat("emergency_hp_restore_amount") }
    pub fn immortality_duration(&self) -> f64 { self.calculate_stat("immortality_duration") }
    pub fn final_redeployment_time(&self) -> f64 { 
        let v = self.calculate_stat("redeployment_time");
        if v > 0.0 { v } else { 0.0 }
    }
    pub fn emergency_hp_restore_uses(&self) -> i32 { 
        let mut has_buff = false;
        for b in self.get_active_buffs() {
            if b.stat == "emergency_hp_restore_uses" { has_buff = true; break; }
        }
        if has_buff { self.calculate_stat("emergency_hp_restore_uses") as i32 } else { 1 }
    }
    pub fn emergency_hp_restore_cooldown(&self) -> f64 { self.calculate_stat("emergency_hp_restore_cooldown") }

    pub fn summon_limit(&self) -> i32 {
        let base = self.base_stats.get("summon_limit").and_then(|v| v.as_f64()).unwrap_or(0.0);
        (self.calculate_stat("summon_limit") + base) as i32
    }

    pub fn immediate_damage_end_instances(&self) -> Vec<ImmediateInstance> {
        let mut inst = Vec::new();
        for b in self.get_active_buffs() {
            if b.stat == "immediate_dmg_end_mult" {
                let val = b.value.as_f64().unwrap_or(0.0);
                let mult = if val > 10.0 { val / 100.0 } else { val };
                inst.push(ImmediateInstance {
                    mult,
                    kind: b.immediate_dmg_kind.clone(),
                    is_true_aoe: b.is_true_aoe,
                });
            }
        }
        inst
    }
    pub fn immediate_hp_loss_end(&self) -> Vec<HpLossInstance> {
        let mut inst = Vec::new();
        for b in self.get_active_buffs() {
            if b.stat == "immediate_hp_loss_end" {
                inst.push(HpLossInstance {
                    value: b.value.as_f64().unwrap_or(0.0),
                    loss_type: b.buff_type.clone(),
                });
            }
        }
        inst
    }
    pub fn immediate_hp_loss_start(&self) -> Vec<HpLossInstance> {
        let mut inst = Vec::new();
        for b in self.get_active_buffs() {
            if b.stat == "immediate_hp_loss_start" {
                inst.push(HpLossInstance {
                    value: b.value.as_f64().unwrap_or(0.0),
                    loss_type: b.buff_type.clone(),
                });
            }
        }
        inst
    }
    pub fn immediate_dp_instances(&self) -> Vec<DpInstance> {
        let mut inst = Vec::new();
        for b in self.get_active_buffs() {
            if b.stat == "immediate_dp" {
                inst.push(DpInstance {
                    value: b.value.as_f64().unwrap_or(0.0),
                });
            }
        }
        inst
    }
    pub fn immediate_damage_instances(&self) -> Vec<ImmediateInstance> {
        let mut inst = Vec::new();
        for b in self.get_active_buffs() {
            if b.stat == "immediate_dmg_mult" {
                let val = b.value.as_f64().unwrap_or(0.0);
                if b.buff_type == "flat" && val > 10.0 { continue; }
                let mult = if val > 10.0 { val / 100.0 } else { val };
                inst.push(ImmediateInstance {
                    mult,
                    kind: b.immediate_dmg_kind.clone(),
                    is_true_aoe: b.is_true_aoe,
                });
            }
        }
        inst
    }
    pub fn immediate_heal_instances(&self) -> Vec<HealInstance> {
        let mut inst = Vec::new();
        for b in self.get_active_buffs() {
            if b.stat == "immediate_heal" {
                inst.push(HealInstance {
                    value: b.value.as_f64().unwrap_or(0.0),
                    heal_type: b.buff_type.clone(),
                    is_true_aoe: b.is_true_aoe,
                });
            }
        }
        inst
    }
    pub fn elemental_buildup_instances(&self) -> Vec<ElementalBuildupInstance> { vec![] }

    pub fn class_id_num(&self) -> Option<i64> {
        self.class_data.as_ref().and_then(|c| c.get("class_id")).and_then(|v| v.as_i64())
    }

    pub fn branch_id_num(&self) -> Option<i64> {
        self.branch_data.as_ref().and_then(|b| b.get("branch_id")).and_then(|v| v.as_i64())
    }

    /// Total damage multiplier per shot from the atk_scale family of blackboard keys.
    /// Values are TOTAL multipliers (e.g. 1.45 = 145% ATK per shot). Buffs are grouped by
    /// source item name; within a group the max scale wins, and a co-existing `prob`
    /// (trigger chance) turns it into an expected value: 1 + p*(s-1). Groups multiply.
    pub fn damage_multiplier(&self) -> f64 {
        let buffs = self.get_active_buffs();
        let is_dollkeeper = self.sub_profession_id == "dollkeeper" || self.subclass_name.contains("傀儡师");
        let mut groups: HashMap<String, (f64, f64)> = HashMap::new(); // name -> (max_scale, prob)
        for b in &buffs {
            // For Dollkeepers, damage_scale in talents/modules is substitute spawn burst on death, not basic attack scale
            if is_dollkeeper && b.stat.contains("damage_scale") && !self.is_skill_active {
                continue;
            }
            let is_scale = (b.stat.contains("atk_scale") && !b.stat.contains("damage_by_atk_scale"))
                || b.stat.contains("damage_scale")
                || b.stat.contains("magic_scale")
                || b.stat.contains("ep_damage_scale");
            if is_scale {
                let mut v = b.value.as_f64().unwrap_or(1.0);
                if (b.stat.contains("damage_scale") || b.stat.contains("magic_scale") || b.stat.contains("ep_damage_scale")) && v < 1.0 && v > 0.0 {
                    v = 1.0 + v;
                }
                let e = groups.entry(b.name.clone()).or_insert((0.0, 1.0));
                if v > e.0 { e.0 = v; }
            } else if b.stat == "prob" {
                let v = b.value.as_f64().unwrap_or(1.0);
                if v > 0.0 && v < 1.0 {
                    let e = groups.entry(b.name.clone()).or_insert((0.0, 1.0));
                    if v < e.1 { e.1 = v; }
                }
            }
        }
        let mut m = 1.0;
        for (_n, (s, p)) in groups {
            if s <= 0.0 { continue; }
            let contrib = 1.0 + p * (s - 1.0);
            if contrib > 0.0 { m *= contrib; }
        }
        m
    }

    pub fn is_primal(&self) -> bool {
        let bid = self.branch_id_num().unwrap_or(0);
        let sub = self.sub_profession_id.to_lowercase();
        let sub_cn = &self.subclass_name;
        bid == 108 || bid == 706 
            || sub.starts_with("prim") 
            || sub == "ritualist" 
            || sub_cn.contains("本源") 
            || sub_cn.contains("巫役")
    }

    pub fn elemental_scale(&self) -> f64 {
        let mut max_scale = 0.0;
        for b in self.get_active_buffs() {
            let s = b.stat.as_str();
            if s.contains("ep_damage") || s.contains("element_damage") || s.contains("element_multiplier") || s.contains("element_atk") {
                let v = b.value.as_f64().unwrap_or(0.0);
                if v > max_scale { max_scale = v; }
            }
        }
        max_scale
    }

    // Deprecated old heal_multiplier removed; use the tuple version defined later.

    /// Flat arts damage per second from poison/corrosion style effects (damage[normal]).
    pub fn flat_arts_dps(&self) -> f64 {
        let mut m = 0.0;
        for b in self.get_active_buffs() {
            if b.stat == "flat_arts_dps" {
                let v = b.value.as_f64().unwrap_or(0.0);
                if v > m { m = v; }
            }
        }
        m
    }

    /// "On skill end" burst multiplier (damage_by_atk_scale), e.g. Blaze S3 detonation.
    pub fn end_burst_mult(&self) -> f64 {
        let mut s = 0.0;
        for b in self.get_active_buffs() {
            if b.stat == "damage_by_atk_scale" {
                s += b.value.as_f64().unwrap_or(0.0);
            }
        }
        s
    }

    /// DP generated per skill activation (blackboard `cost` on Vanguard skills, `value` on Flagbearers, ammo/timed on Agents).
    pub fn dp_gain_per_cast(&self) -> f64 {
        if let Some(s) = &self.equipped_skill {
            let mut dp: f64 = 0.0;
            let desc = &s.description;
            let is_dp_skill = desc.contains("获得") && (desc.contains("费用") || desc.contains("Cost") || desc.contains("DP"))
                || s.name.contains("支援") || s.name.contains("Order") || s.name.contains("Command")
                || desc.contains("回复总共") || desc.contains("恢复总共");
            
            let mut cost_val: f64 = 0.0;
            let mut val_val: f64 = 0.0;
            let mut ammo_val: f64 = 0.0;
            for b in &s.buffs {
                if b.stat == "dp_gain_per_cast" {
                    dp = dp.max(b.value.as_f64().unwrap_or(0.0));
                }
                if b.stat == "cost" {
                    cost_val = cost_val.max(b.value.as_f64().unwrap_or(0.0));
                }
                if b.stat == "value" {
                    val_val = val_val.max(b.value.as_f64().unwrap_or(0.0));
                }
                if b.stat.contains("trigger_time") || b.stat.contains("ammo") {
                    ammo_val = ammo_val.max(b.value.as_f64().unwrap_or(0.0));
                }
            }

            // 1. Flagbearer total DP skills (Myrtle: 14, Elysium: 18/20, Saileach: 18/20)
            if val_val > 0.0 && (is_dp_skill || cost_val > 0.0 || dp > 0.0) {
                return val_val;
            }

            // 2. Agent ammo skills (e.g. Cantabile S2: 18 ammo * 1 DP = 18 DP)
            if ammo_val > 0.0 && (cost_val > 0.0 || dp > 0.0) {
                let unit = if cost_val > 0.0 { cost_val } else { dp };
                return ammo_val * unit;
            }

            // 3. Agent timed skills (e.g. Ines S2: 12s duration, attacks every ~0.8-1.0s * 1 DP = 12-14 DP)
            if s.duration > 0.0 && is_dp_skill && (cost_val > 0.0 && cost_val <= 2.0 || dp > 0.0 && dp <= 2.0) {
                let unit = if cost_val > 0.0 { cost_val } else { dp };
                let int = self.final_interval().max(0.4);
                return (s.duration / int).floor() * unit;
            }

            // 4. Direct DP gain per cast (Pioneer skills like Texas 12, Siege 12/3, Courier 9, Flametail 13, SilverAsh Reignfrost 10/11)
            let direct = dp.max(cost_val);
            if direct > 0.0 {
                return direct;
            }
        }
        let mut s = 0.0;
        for b in self.get_active_buffs() {
            if b.stat == "dp_gain_per_cast" {
                let v = b.value.as_f64().unwrap_or(0.0);
                if v > 0.0 { s += v; }
            }
        }
        s
    }

    pub fn damage_resistance(&self) -> f64 {
        let mut max_res = 0.0;
        for b in self.get_active_buffs() {
            if b.stat == "damage_resistance" || b.stat == "sanctuary" || b.stat == "damage_reduction" {
                let mut v = b.value.as_f64().unwrap_or(0.0).abs();
                if v >= 5.0 { v /= 100.0; }
                if v > max_res { max_res = v; }
            }
        }
        max_res.clamp(0.0, 0.70)
    }

    pub fn initial_barrier(&self) -> f64 {
        let hp = self.final_hp();
        let mut b: f64 = 0.0;
        for t in &self.talents {
            for buff in &t.buffs {
                if buff.stat == "born_hp_ratio" {
                    let v = buff.value.as_f64().unwrap_or(0.0);
                    if v > 0.0 { b = b.max(hp * v); }
                }
            }
        }
        if let Some(m) = &self.active_module {
            for buff in &m.buffs {
                if buff.stat == "born_hp_ratio" {
                    let v = buff.value.as_f64().unwrap_or(0.0);
                    if v > 0.0 { b = b.max(hp * v); }
                }
            }
        }
        for buff in self.get_active_buffs() {
            if buff.stat == "barrier" || buff.stat == "shield" {
                let v = buff.value.as_f64().unwrap_or(0.0);
                if buff.buff_type == "ratio" { b = b.max(hp * v); }
                else if v > 0.0 { b = b.max(v); }
            }
        }
        b
    }

    pub fn skill_barrier(&self) -> f64 {
        let hp = self.final_hp();
        if let Some(s) = &self.equipped_skill {
            let desc = s.description.to_lowercase();
            if desc.contains("barrier") || desc.contains("shield") || desc.contains("护盾") {
                for b in &s.buffs {
                    if b.stat == "hp_ratio" || b.stat == "barrier" || b.stat == "shield" {
                        let v = b.value.as_f64().unwrap_or(0.0);
                        if v > 0.0 {
                            return if b.buff_type == "flat" { v } else { hp * v };
                        }
                    }
                }
            }
        }
        0.0
    }

    pub fn max_barrier_cap(&self) -> f64 {
        let hp = self.final_hp();
        let mut cap = hp;
        for t in &self.talents {
            for b in &t.buffs {
                if b.stat == "max_hp_ratio" {
                    let v = b.value.as_f64().unwrap_or(0.0);
                    if v > 0.0 { cap = cap.max(hp * v); }
                }
            }
        }
        if let Some(m) = &self.active_module {
            for b in &m.buffs {
                if b.stat == "max_hp_ratio" {
                    let v = b.value.as_f64().unwrap_or(0.0);
                    if v > 0.0 { cap = cap.max(hp * v); }
                }
            }
        }
        cap
    }

    pub fn thorns_reflect_ratio(&self) -> f64 {
        for t in &self.talents {
            let d = t.description.to_lowercase();
            if (d.contains("attacked") || d.contains("受到攻击") || d.contains("under the effect of her own barrier") || d.contains("护盾"))
                && (d.contains("arts damage") || d.contains("法术伤害") || d.contains("thorns") || d.contains("荆棘")) {
                for b in &t.buffs {
                    if b.stat == "atk_scale" {
                        return b.value.as_f64().unwrap_or(0.53);
                    }
                }
                return 0.53;
            }
        }
        0.0
    }
    pub fn calculate_ehp_phys_against(&self, enemy_atk: f64) -> f64 {
        let hp = self.final_hp();
        if hp <= 0.0 { return 0.0; }
        let barrier = self.initial_barrier() + if self.is_skill_active { self.skill_barrier() } else { 0.0 };
        let regen = self.hp_regen_per_second() * 10.0;
        let total_pool = (hp + barrier + regen).min((hp * 2.5) + self.max_barrier_cap());
        let def = self.final_def().max(0.0);
        let e_atk = enemy_atk.max(1200.0);
        let dmg_taken = f64::max(0.05 * e_atk, e_atk - def);
        let hits_survived = (total_pool / dmg_taken).clamp(1.0, 20.0);
        let dmg_resist = (1.0 - self.damage_resistance()).clamp(0.20, 1.0);
        (hits_survived * e_atk) / dmg_resist
    }

    pub fn calculate_ehp_arts_against(&self, _enemy_atk: f64) -> f64 {
        let hp = self.final_hp();
        if hp <= 0.0 { return 0.0; }
        let barrier = self.initial_barrier() + if self.is_skill_active { self.skill_barrier() } else { 0.0 };
        let regen = self.hp_regen_per_second() * 10.0;
        let total_pool = (hp + barrier + regen).min((hp * 2.5) + self.max_barrier_cap());
        let magic_res = self.calculate_stat("res").clamp(0.0, 90.0);
        let dmg_resist = (1.0 - self.damage_resistance()).clamp(0.20, 1.0);
        let ehp = total_pool / (1.0 - (magic_res / 100.0)).max(0.10);
        ehp / dmg_resist
    }

    pub fn calculate_ehp_phys(&self) -> f64 {
        self.calculate_ehp_phys_against(1200.0)
    }

    pub fn calculate_ehp_arts(&self) -> f64 {
        self.calculate_ehp_arts_against(1200.0)
    }

    pub fn is_true_aoe(&self) -> bool {
        let sub = self.sub_profession_id.to_lowercase();
        let sub_cn = &self.subclass_name;
        let is_base_aoe = matches!(sub.as_str(), "splashcaster" | "blastcaster" | "phalanx" | "aoesniper" | "bombarder" | "reaperrange" | "shotgun" | "flinger" | "reaper" | "fortress")
            || sub_cn.contains("轰击") || sub_cn.contains("扩散") || sub_cn.contains("阵法") || sub_cn.contains("攻城") || sub_cn.contains("散射") || sub_cn.contains("投掷") || sub_cn.contains("收割") || sub_cn.contains("要塞");

        if is_base_aoe { return true; }

        if let Some(skill) = &self.equipped_skill {
            let desc = &skill.description;
            let s_name = &skill.name;
            if skill.is_true_aoe 
                || desc.contains("群体攻击") 
                || desc.contains("变为群体") 
                || desc.contains("所有敌人") 
                || desc.contains("所有敌方单位") 
                || desc.contains("对附近所有") 
                || desc.contains("对周围所有") 
                || desc.contains("溅射") 
                || desc.contains("群体法术")
                || s_name == "众恶的焚场" 
                || s_name == "点燃" 
                || s_name == "照明榴弹" 
                || s_name == "我无" 
                || s_name == "真银斩" 
                || s_name == "火山" 
                || s_name == "未照耀的荣光"
            { 
                return true; 
            }
        }
        if let Some(module) = &self.active_module {
            if module.is_true_aoe { return true; }
        }
        for b in self.get_active_buffs() {
            if b.is_true_aoe { return true; }
        }
        false
    }

    pub fn target_limit(&self) -> f64 {
        let bid = self.branch_id_num().unwrap_or(0);
        let sub = self.sub_profession_id.to_lowercase();
        let sub_cn = &self.subclass_name;

        // Branches whose basic attack hits multiple enemies (classes.json descriptions)
        let aoe_branches = [101, 106, 107, 203, 310, 501, 504, 508, 601, 607];
        let is_base_aoe = aoe_branches.contains(&bid) 
            || matches!(sub.as_str(), "splashcaster" | "blastcaster" | "phalanx" | "aoesniper" | "bombarder" | "reaperrange" | "shotgun" | "flinger" | "reaper" | "fortress")
            || sub_cn.contains("轰击") || sub_cn.contains("扩散") || sub_cn.contains("阵法") || sub_cn.contains("攻城") || sub_cn.contains("散射") || sub_cn.contains("投掷") || sub_cn.contains("收割") || sub_cn.contains("要塞");

        let is_centurion = bid == 302 || sub == "centurion" || sub_cn.contains("强攻");
        let is_crusher = bid == 303 || sub == "crusher" || sub_cn.contains("重剑");
        let is_chain = sub == "chain" || sub == "chainhealer" || sub_cn.contains("链");

        let mut is_skill_aoe = false;
        if self.is_skill_active {
            if let Some(skill) = &self.equipped_skill {
                let desc = &skill.description;
                let s_name = &skill.name;
                if skill.is_true_aoe 
                    || desc.contains("群体攻击") 
                    || desc.contains("变为群体") 
                    || desc.contains("所有敌人") 
                    || desc.contains("所有敌方单位") 
                    || desc.contains("对附近所有") 
                    || desc.contains("对周围所有") 
                    || desc.contains("溅射") 
                    || desc.contains("群体法术")
                    || s_name == "众恶的焚场" 
                    || s_name == "点燃" 
                    || s_name == "照明榴弹" 
                    || s_name == "我无" 
                    || s_name == "真银斩" 
                    || s_name == "火山" 
                    || s_name == "未照耀的荣光"
                { 
                    is_skill_aoe = true; 
                }
            }
        }
        if let Some(module) = &self.active_module {
            if module.is_true_aoe { is_skill_aoe = true; }
        }

        let base = if is_base_aoe || is_skill_aoe {
            5.0
        } else if is_centurion {
            let block = self.base_stats.get("block_count").and_then(|v| v.as_f64()).unwrap_or(3.0);
            (block + self.calculate_stat("block_count")).max(3.0)
        } else if is_crusher {
            2.0
        } else if is_chain {
            4.0
        } else {
            1.0
        };

        let mut limit = base + self.calculate_stat("target_limit");

        // Absolute target counts and additions from buffs
        for b in self.get_active_buffs() {
            let s = b.stat.as_str();
            let v = b.value.as_f64().unwrap_or(0.0);
            if s == "target_abs" || (s.contains("max_target") && !s.contains("add") && !s.contains("extra")) {
                if v > limit { limit = v; }
            } else if s.contains("max_target") && (s.contains("add") || s.contains("extra")) {
                limit += v;
            }
        }
        if limit > 1.0 { limit } else { 1.0 }
    }

    pub fn base_block_count(&self) -> f64 {
        self.base_stats.get("block_count").and_then(|v| v.as_f64()).unwrap_or(0.0)
    }

    pub fn total_field_block(&self) -> f64 {
        let mut total = self.calculate_stat("block_count").max(0.0);

        // Include summons' block capability
        if !self.summons.is_empty() {
            let mut s_block = 0.0;
            for s in &self.summons {
                s_block += s.base_stats.get("block_count").and_then(|v| v.as_f64()).unwrap_or(0.0);
            }
            let avg_s = s_block / (self.summons.len() as f64);
            let limit = (self.summon_limit().max(1) as f64).min(2.0);
            total += avg_s * limit;
        }
        total
    }

    pub fn hits_mult(&self) -> f64 {
        let bid = self.branch_id_num().unwrap_or(0);
        // Absolute shot counts from buffs (`times` family) override everything
        let mut abs = 0.0;
        for b in self.get_active_buffs() {
            // Ignore stacking talent/module buffs where 'times' means stack count or shield stacks, not hits per attack
            let is_stack_buff = b.name.contains("佣兵之韧") 
                || b.name.contains("干明可鉴") 
                || b.name.contains("沃土予身") 
                || b.name.contains("MSC-") 
                || b.name.contains("PRO-X") 
                || b.name.contains("UNY-X")
                || b.name.contains("AFT-Y")
                || self.sub_profession_id == "artsfghter"
                || self.sub_profession_id == "mystic"
                || self.sub_profession_id == "pioneer";
            if is_stack_buff && b.stat == "times" {
                continue;
            }

            if b.stat == "hits_abs"
                || b.stat == "attack@times"
                || b.stat.ends_with("@times")
                || b.stat == "multi_times"
                || (b.stat == "times" && self.is_skill_active)
            {
                let v = b.value.as_f64().unwrap_or(0.0);
                if v > abs { abs = v; }
            }
        }
        if abs >= 1.0 { return abs; }
        let mut base_mult = 1.0;
        // Swordmaster (311) attacks twice; Flinger (504) projectiles hit twice
        if bid == 311 || bid == 504 {
            base_mult = 2.0;
        }
        let mult = base_mult + self.calculate_stat("hits_mult");
        if mult > 1.0 { mult } else { 1.0 }
    }

    pub fn healing_mult(&self) -> f64 {
        let hm = self.calculate_stat("healing_mult");
        let base = if hm > 0.0 { hm } else { 1.0 };
        base * (1.0 + self.calculate_stat("healing_bonus"))
    }

    pub fn heal_multiplier(&self) -> (f64, bool) {
        let mut m = 0.0;
        let mut applies_to_allies = false;
        for b in self.get_active_buffs() {
            if b.stat.contains("heal_scale") || b.stat == "atk_to_hp_recovery_ratio" {
                let v = b.value.as_f64().unwrap_or(0.0);
                if v > m { 
                    m = v; 
                    applies_to_allies = b.applies_to_allies;
                }
            }
        }
        (m, applies_to_allies)
    }

    pub fn mech_accord_drones(&self) -> f64 { self.calculate_stat("mech_accord_drones") }
    pub fn mech_accord_dmg_kind(&self) -> String {
        for b in self.get_active_buffs() {
            if b.stat == "mech_accord_dmg_kind" {
                return b.immediate_dmg_kind.clone();
            }
        }
        "arts".to_string()
    }
    pub fn mech_accord_initial_mult(&self) -> f64 {
        let v = self.calculate_stat("mech_accord_initial_mult");
        if v > 0.0 { v } else { 0.20 }
    }
    pub fn mech_accord_max_mult(&self) -> f64 {
        let v = self.calculate_stat("mech_accord_max_mult");
        if v > 0.0 { v } else { 1.10 }
    }

    pub fn get_position(&self) -> String {
        if self.position == "MELEE" || self.position == "RANGED" {
            return self.position.clone();
        }
        
        // Infer from profession and subclass
        let prof = self.profession.to_uppercase();
        let sub = self.sub_profession_id.to_lowercase();
        
        match prof.as_str() {
            "WARRIOR" | "TANK" | "DEFENDER" | "PIONEER" => "MELEE".to_string(),
            "SNIPER" | "CASTER" | "MEDIC" | "SUPPORT" | "SUPPORTER" => "RANGED".to_string(),
            "SPECIAL" => {
                if sub == "geek" || sub == "trapmaster" || sub == "traper" || self.is_trapmaster() {
                    "RANGED".to_string()
                } else {
                    "MELEE".to_string()
                }
            },
            _ => "MELEE".to_string(), // Default fallback
        }
    }

    pub fn apply_decay_internal(&self) -> bool { true }

    pub fn attack_overrides(&self) -> Vec<AttackOverride> {
        let mut overrides = Vec::new();
        for b in self.get_active_buffs() {
            if b.stat.ends_with("atk_scale") {
                overrides.push(AttackOverride {
                    value: AttackOverrideValue {
                        dmg_kind: "physical".to_string(), // will be overridden by current_action_params
                        atk_mod: b.value.as_f64().unwrap_or(1.0),
                        action_type: "damage".to_string(),
                        heal_mod: 0.0,
                        dmg_ratio: 0.0,
                        dmg_flat: 0.0,
                        combo_attacks: None,
                    }
                });
            }
        }
        overrides
    }

    pub fn extra_damage_on_hit(&self) -> Vec<ImmediateInstance> {
        let mut inst = Vec::new();
        for b in self.get_active_buffs() {
            if b.stat == "extra_damage_on_hit" {
                let mut val = b.current_decay_value;
                if val == 0.0 { val = b.value.as_f64().unwrap_or(0.0); }
                inst.push(ImmediateInstance {
                    mult: val,
                    kind: b.immediate_dmg_kind.clone(),
                    is_true_aoe: false,
                });
            } else if b.stat == "immediate_dmg_mult" && b.buff_type == "flat" {
                let val = b.value.as_f64().unwrap_or(0.0);
                if val > 10.0 {
                    inst.push(ImmediateInstance {
                        mult: val / 10.0,
                        kind: b.immediate_dmg_kind.clone(),
                        is_true_aoe: false,
                    });
                }
            }
        }
        inst
    }

    pub fn current_action_params(&self) -> (String, f64, f64, String) {
        let mut act = "damage".to_string();
        let mut d_m = 1.0;
        let mut h_m = 0.0;
        let mut kind = "physical".to_string();
        
        let c_id = self.class_data.as_ref().and_then(|c| c.get("class_id")).and_then(|i| i.as_i64());
        let b_id = self.branch_data.as_ref().and_then(|c| c.get("branch_id")).and_then(|i| i.as_i64());
        
        if c_id == Some(4) {
            act = "heal".to_string();
            d_m = 0.0;
            h_m = 1.0;
            if b_id == Some(405) {
                act = "both".to_string();
                d_m = 1.0;
                h_m = 0.5;
            }
        }
        
        if [Some(301), Some(405)].contains(&b_id) || ([Some(1), Some(7)].contains(&c_id) && ![Some(702), Some(703)].contains(&b_id)) {
            kind = "arts".to_string();
        }
        
        for t in &self.talents {
            if t.name.contains("Potential") { continue; }
            if t.action_type != "none" {
                act = t.action_type.clone();
                d_m = t.dmg_mult;
                h_m = t.heal_mult;
                kind = t.dmg_kind.clone();
            }
        }
        
        if let Some(skill) = &self.equipped_skill {
            if self.is_skill_active || skill.is_passive() {
                if skill.action_type != "none" {
                    act = skill.action_type.clone();
                    d_m = skill.dmg_mult;
                    h_m = skill.heal_mult;
                    kind = skill.dmg_kind.clone();
                }
            }
        }
        
        if self.calculate_stat("true_damage") > 0.0 {
            kind = "true".to_string();
        } else if self.calculate_stat("weakness_damage") > 0.0 {
            kind = "weakness".to_string();
        }
        
        if act == "heal" { d_m = 0.0; }
        else if act == "damage" { h_m = 0.0; }
        else if act == "none" { d_m = 0.0; h_m = 0.0; }

        for b in self.get_active_buffs() {
            let val = b.value.as_f64().unwrap_or(1.0);
            if b.stat.ends_with("atk_scale") || b.stat.ends_with("damage_scale") || b.stat.ends_with("damage_scale_post") {
                if val != 0.0 { d_m *= val; }
            } else if b.stat.ends_with("heal_scale") {
                if val != 0.0 { h_m *= val; }
            }
        }
        
        (act, d_m, h_m, kind)
    }

    pub fn is_chain_medic(&self) -> bool {
        self.branch_data.as_ref().and_then(|b| b.get("branch_id")).and_then(|v| v.as_i64()) == Some(406)
    }
    pub fn chain_no_weaken(&self) -> bool { self.calculate_stat("chain_no_weaken") > 0.5 }
    pub fn chain_bounce_penalty(&self) -> f64 {
        let p = self.calculate_stat("chain_penalty_fixed");
        if p > 0.0 { p } else { 0.25 }
    }
    pub fn is_multitarget_medic(&self) -> bool {
        self.branch_data.as_ref().and_then(|b| b.get("branch_id")).and_then(|v| v.as_i64()) == Some(402)
    }
    pub fn heal_from_damage(&self) -> f64 { self.calculate_stat("heal_from_damage") }

    pub fn is_abjurer(&self) -> bool {
        let bid = self.branch_id_num().unwrap_or(0);
        let sub = self.sub_profession_id.to_lowercase();
        bid == 701 || sub == "blessing" || self.subclass_name == "Abjurer" || self.subclass_name.contains("护佑")
    }

    pub fn is_guardian(&self) -> bool {
        let bid = self.branch_id_num().unwrap_or(0);
        let sub = self.sub_profession_id.to_lowercase();
        bid == 204 || sub == "guardian" || self.subclass_name == "Guardian" || self.subclass_name.contains("守护")
    }

    pub fn is_bard(&self) -> bool {
        let bid = self.branch_id_num().unwrap_or(0);
        let sub = self.sub_profession_id.to_lowercase();
        bid == 703 || sub == "bard" || self.subclass_name == "Bard" || self.subclass_name.contains("吟游")
    }

    pub fn is_incantation_medic(&self) -> bool {
        let bid = self.branch_id_num().unwrap_or(0);
        let sub = self.sub_profession_id.to_lowercase();
        bid == 405 || sub == "incantationmedic" || self.subclass_name == "Incantation Medic" || self.subclass_name.contains("咒愈")
    }

    pub fn is_wandering_medic(&self) -> bool {
        let bid = self.branch_id_num().unwrap_or(0);
        let sub = self.sub_profession_id.to_lowercase();
        bid == 404 || sub == "wandermedic" || self.subclass_name == "Wandering Medic" || self.subclass_name.contains("行医")
    }

    pub fn is_medic(&self) -> bool {
        let cid = self.class_id_num().unwrap_or(0);
        cid == 4 || self.profession == "MEDIC"
    }

    pub fn is_team_healer(&self) -> bool {
        self.is_medic() || self.is_abjurer() || self.is_guardian() || self.is_bard()
    }

    pub fn is_trapmaster(&self) -> bool {
        let bid = self.branch_id_num().unwrap_or(0);
        let sub = self.sub_profession_id.to_lowercase();
        bid == 608 || sub == "trapmaster" || sub == "traper" || self.subclass_name == "Trapmaster" || self.subclass_name.contains("陷阱")
    }

    pub fn is_executor(&self) -> bool {
        let bid = self.branch_id_num().unwrap_or(0);
        let sub = self.sub_profession_id.to_lowercase();
        bid == 603 || sub == "executor" || self.subclass_name == "Executor" || self.subclass_name.contains("处决")
    }

    pub fn is_defender(&self) -> bool {
        let cid = self.class_id_num().unwrap_or(0);
        let prof = self.profession.to_uppercase();
        cid == 2 || prof == "TANK" || prof == "DEFENDER"
    }

    pub fn calculate_hits_to_kill(&self, enemy_atk: f64, enemy_interval: f64, arts_ratio: f64) -> f64 {
        let hp = self.final_hp().max(1.0);
        let barrier = self.initial_barrier() + self.skill_barrier();
        let regen_per_sec = self.hp_regen_per_second();
        let regen_buffer = regen_per_sec * 10.0;
        let total_pool = hp + barrier + regen_buffer;
        let def = self.final_def().max(0.0);
        let res = self.calculate_stat("res").clamp(0.0, 95.0);
        let res_mult = 1.0 - (res / 100.0);
        let dmg_resist = (1.0 - self.damage_resistance()).clamp(0.05, 1.0);

        let phys_dmg = (enemy_atk * (1.0 - arts_ratio) - def).max(0.05 * enemy_atk) * dmg_resist;
        let arts_dmg = (enemy_atk * arts_ratio * res_mult) * dmg_resist;
        let raw_incoming_per_hit = (phys_dmg + arts_dmg).max(1.0);

        // Self-heal / regen per hit interval
        let regen_per_hit = regen_per_sec * enemy_interval.max(0.5);
        let net_dmg_per_hit = raw_incoming_per_hit - regen_per_hit;

        if net_dmg_per_hit <= (0.05 * enemy_atk * dmg_resist) {
            // Regeneration overcomes or matches incoming damage floor
            50.0
        } else {
            (total_pool / net_dmg_per_hit).clamp(1.0, 50.0)
        }
    }
}

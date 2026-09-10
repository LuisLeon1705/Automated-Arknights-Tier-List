#![allow(dead_code)]
use std::collections::HashMap;
use super::models::Operator;

pub struct SimulationEnvironment {
    pub primary_operator: Operator,
    pub other_operators: Vec<Operator>,
    pub target_stats: HashMap<String, f64>,
    pub duration: f64,
}

/// Raw (unmitigated) output rates of the operator in one state (base or skill-active).
#[derive(Clone, Copy)]
pub struct StateRates {
    pub atk: f64,
    pub attacks_per_sec: f64,
    pub shots_per_attack: f64,
    pub phys_per_shot: f64,
    pub arts_per_shot: f64,
    pub true_per_shot: f64,
    pub elemental_per_shot: f64,
    pub heal_per_sec: f64,
    pub heal_targets: f64,
    pub flat_arts_dps: f64,
    pub flat_ele_dps: f64,
    pub heal_from_arts: f64,
    pub end_burst_raw: f64,
    pub target_limit: f64,
}

impl SimulationEnvironment {
    pub fn new(
        primary_operator: Operator,
        other_operators: Option<Vec<Operator>>,
        target_stats: Option<HashMap<String, f64>>,
    ) -> Self {
        let other_operators = other_operators.unwrap_or_default();
        let mut t_stats = target_stats.unwrap_or_default();
        if !t_stats.contains_key("def") { t_stats.insert("def".to_string(), 0.0); }
        if !t_stats.contains_key("res") { t_stats.insert("res".to_string(), 0.0); }
        if !t_stats.contains_key("is_boss") { t_stats.insert("is_boss".to_string(), 0.0); }
        if !t_stats.contains_key("weight") { t_stats.insert("weight".to_string(), 0.0); }
        if !t_stats.contains_key("hp") { t_stats.insert("hp".to_string(), 10000.0); }
        if !t_stats.contains_key("atk") { t_stats.insert("atk".to_string(), 500.0); }
        if !t_stats.contains_key("attack_interval") { t_stats.insert("attack_interval".to_string(), 3.0); }

        Self {
            primary_operator,
            other_operators,
            target_stats: t_stats,
            duration: 300.0,
        }
    }

    /// Computes raw rates for the current `is_skill_active` state of the primary operator,
    /// using class/branch semantics from classes.json plus blackboard multipliers
    /// (atk_scale family, times family, heal_scale family).
    pub fn state_rates(&self) -> StateRates {
        let op = &self.primary_operator;
        let bid = op.branch_id_num().unwrap_or(0);
        let cid = op.class_id_num().unwrap_or(0);
        let is_skill = op.is_skill_active;

        let is_medic = op.is_medic();
        let is_abjurer = op.is_abjurer();
        let is_bard = op.is_bard();
        let is_guardian = op.is_guardian();
        let incantation = op.is_incantation_medic();
        let arts = matches!(bid, 101..=107 | 201 | 301 | 405 | 701 | 702 | 704..=707)
            || (bid == 0 && (cid == 1 || cid == 7 || op.profession == "CASTER" || op.profession == "SUPPORT" || is_abjurer || is_bard || incantation));
        // Phalanx (106) and Liberator (307) do not attack while their skill is inactive
        let no_base_attack = matches!(bid, 106 | 307);

        let atk = op.final_atk().max(0.0);
        let interval = op.final_interval().max(0.1);
        let attacks_per_sec = 1.0 / interval;
        let shots = op.hits_mult();
        let dmg_mult = op.damage_multiplier();
        let (heal_buff, heal_applies_to_allies) = op.heal_multiplier();

        let mut r = StateRates {
            atk,
            attacks_per_sec,
            shots_per_attack: shots,
            phys_per_shot: 0.0,
            arts_per_shot: 0.0,
            true_per_shot: 0.0,
            elemental_per_shot: 0.0,
            heal_per_sec: 0.0,
            heal_targets: 1.0,
            flat_arts_dps: 0.0,
            flat_ele_dps: 0.0,
            heal_from_arts: 0.0,
            end_burst_raw: 0.0,
            target_limit: op.target_limit(),
        };

        if !is_skill && no_base_attack {
            return r;
        }

        // -1. TRAPMASTER (Branch 608: Dorothy, Wang, Ela, Frost, Robin, Wulfenite)
        if op.is_trapmaster() {
            let s_name = op.equipped_skill.as_ref().map(|s| s.name.as_str()).unwrap_or("");
            let s_desc = op.equipped_skill.as_ref().map(|s| s.description.as_str()).unwrap_or("");
            let s_cost = op.equipped_skill.as_ref().map(|s| s.sp_cost).unwrap_or(12.0).max(5.0);
            
            // In a 300s encounter, Trapmasters start with ~10 traps and regenerate 1 trap every s_cost seconds
            let total_traps = 10.0 + (300.0 / s_cost);
            let traps_per_sec = total_traps / 300.0; // ~0.11 - 0.18 traps/sec

            if op.name.contains("Dorothy") || op.is_char("char_4048_doroth") {
                if s_name.contains("快速起爆") || s_name.contains("Quick Detonation") {
                    // S1: 450% ATK Physical damage + 35% DEF shred
                    let trap_phys_dps = (atk * 4.5) * traps_per_sec;
                    r.attacks_per_sec = 1.0 / interval;
                    r.phys_per_shot = atk + (trap_phys_dps / r.attacks_per_sec);
                } else if s_name.contains("微震约束") || s_name.contains("Micro-Vibration") {
                    // S2: 380% ATK Physical AoE + 3.5s Bind (avg 2 targets)
                    let trap_phys_dps = (atk * 3.8 * 2.0) * traps_per_sec;
                    r.attacks_per_sec = 1.0 / interval;
                    r.phys_per_shot = atk + (trap_phys_dps / r.attacks_per_sec);
                } else {
                    // S3 (Resonance): 350% ATK Arts damage in cross AoE + chain trigger (avg 2.5 targets) + slow
                    let trap_arts_dps = (atk * 3.5 * 2.5) * traps_per_sec;
                    r.phys_per_shot = atk;
                    r.attacks_per_sec = 1.0 / interval;
                    r.flat_arts_dps += trap_arts_dps;
                }
            } else if op.name.contains("Ela") || op.is_char("char_1034_ela") {
                // Ela: Booby Traps inflict 40% Fragile + S3 rapid-fire burst
                if is_skill {
                    r.phys_per_shot = atk * 2.5;
                    r.attacks_per_sec = 1.0 / 0.40; // 0.4s attack interval with shotgun
                } else {
                    r.phys_per_shot = atk;
                    r.attacks_per_sec = 1.0 / interval;
                }
                let trap_dps = (atk * 2.5) * traps_per_sec;
                r.phys_per_shot += trap_dps / r.attacks_per_sec;
            } else if op.name.contains("Wang") || op.is_char("char_2027_wang") || op.is_char("char_1033_wang") || op.name.contains("望") {
                // Wang (Trapmaster, char_2027_wang): Traps deal massive ARTS DAMAGE with +36% line bonus and ignore 30 Arts RES!
                let talent2_mult = 1.36;
                if is_skill {
                    if s_name.contains("天下劫") || s_name.contains("3") || s_desc.contains("弹药") {
                        // S3: 20 ammo, drops 8 initial traps, each drop spawns up to 3 extra chain traps (4 traps/cluster)
                        // Detonations deal 380% ATK Arts damage with 36% bonus and ignore 30 Arts RES!
                        r.attacks_per_sec = 1.0 / 1.0;
                        r.shots_per_attack = 4.0;
                        r.arts_per_shot = atk * 3.8 * talent2_mult;
                        r.phys_per_shot = 0.0;
                        r.target_limit = 5.0;
                    } else if s_name.contains("连星") || s_name.contains("2") {
                        // S2: Cross explosion dealing 580% ATK Arts damage + 50% slow
                        r.attacks_per_sec = 1.0 / 1.5;
                        r.shots_per_attack = 1.0;
                        r.arts_per_shot = atk * 5.8 * talent2_mult;
                        r.phys_per_shot = 0.0;
                        r.target_limit = 4.0;
                    } else {
                        // S1: 135% ATK/s Arts damage for 6.5s + Sluggish
                        r.attacks_per_sec = 1.0 / 1.2;
                        r.shots_per_attack = 1.0;
                        r.arts_per_shot = atk * 1.35 * 6.5 * talent2_mult;
                        r.phys_per_shot = 0.0;
                        r.target_limit = 2.0;
                    }
                } else {
                    // Base state: Wang attacks with basic attack + passive trap detonations (1 trap every ~2.5s)
                    r.attacks_per_sec = 1.0 / interval;
                    r.shots_per_attack = 1.0;
                    r.phys_per_shot = atk;
                    r.arts_per_shot = (atk * 3.8 * talent2_mult) / (2.5 / interval);
                    r.target_limit = 2.0;
                }
            } else {
                // Frost / Robin / Wulfenite
                let trap_dps = (atk * 3.5) * traps_per_sec;
                r.attacks_per_sec = 1.0 / interval;
                r.phys_per_shot = atk + (trap_dps / r.attacks_per_sec);
            }
            return r;
        }

        // 0. INCANTATION MEDIC (Branch 405, e.g. Reed the Flame Shadow, Hibiscus the Purifier)
        if incantation {
            let mult = if dmg_mult > 0.0 { dmg_mult } else { 1.0 };
            r.arts_per_shot = atk * mult;
            r.heal_from_arts = 0.50; // Converts 50% damage into healing
            if is_skill {
                if let Some(s) = &op.equipped_skill {
                    if s.name.contains("生命之火") || s.name.contains("Flame of Life") {
                        r.target_limit = 2.0;
                    }
                }
            }
            return r;
        }

        // 1. ABJURER (Branch 701, e.g. Haruka, Quercus, Silence the Paradigmatic)
        if is_abjurer {
            let is_haruka = op.name == "Haruka" || op.is_char("char_4202_haruka");
            let bubble_heal_ratio = if is_haruka { 0.31 } else { 0.0 };
            
            if is_skill {
                // When skill is active, Abjurers stop attacking enemies and heal allies for 75% ATK
                let mut heal_targets = 1.0;
                let mut bubble_targets = 1.0;
                if let Some(s) = &op.equipped_skill {
                    if s.name == "幽隙栖萤" || s.description.contains("治疗目标数+1") {
                        heal_targets = 2.0;
                        bubble_targets = 2.0;
                    }
                }
                let hm = if heal_buff >= 0.75 { heal_buff } else { 0.75 };
                let direct_hps = heal_targets * atk * hm / interval;
                let bubble_hps = bubble_targets * atk * bubble_heal_ratio / interval;
                r.heal_per_sec = direct_hps + bubble_hps;
                r.heal_targets = heal_targets;
                r.arts_per_shot = 0.0;
                return r;
            } else {
                // Base state: deals Arts damage, and bubbles passively heal allies
                r.arts_per_shot = atk * dmg_mult;
                r.heal_per_sec = atk * bubble_heal_ratio / interval;
                return r;
            }
        }

        // 2. BARD (Branch 703, e.g. Skadi Alter)
        if is_bard {
            let hm = if heal_buff > 0.0 { heal_buff } else { 0.25 };
            r.heal_per_sec = atk * hm / 1.0;
            r.heal_targets = 3.0;
            return r;
        }

        // 3. MEDIC (Branches 401, 402, 403, 404, 406, etc.)
        if is_medic && !incantation {
            let is_lumen = op.name == "Lumen";
            let mut hm = if is_lumen {
                // Lumen S3 heal_scale: 2.0 is only for the 8 status purge shots; normal healing is 1.0
                1.0
            } else if heal_buff >= 0.5 { 
                heal_buff 
            } else { 
                1.0 
            };
            let mut targets: f64 = 1.0;

            let sub_id = op.sub_profession_id.to_lowercase();
            let sub_cn = &op.subclass_name;

            // Multi-target Medic (402, Nightingale, Ptilopsis)
            if bid == 402 || sub_id == "ringhealer" || sub_cn == "Multi-target Medic" || sub_cn.contains("群愈") {
                targets = 3.0;
            }
            // Chain Medic (406, Mon3tr)
            else if bid == 406 || sub_id == "chainhealer" || sub_cn == "Chain Medic" || sub_cn.contains("链愈") {
                let is_mon3tr = op.name == "Mon3tr";
                if is_mon3tr && is_skill {
                    if let Some(s) = &op.equipped_skill {
                        if s.name.contains("熔毁") {
                            // S3 attacks enemies with True damage and heals like an Incantation Medic (ReedAlter)
                            r.phys_per_shot = 0.0;
                            r.arts_per_shot = 0.0;
                            r.true_per_shot = atk * dmg_mult;
                            r.heal_from_arts = 0.5; // Converts 50% of damage dealt into healing
                            r.heal_per_sec = 0.0;
                            return r;
                        } else if s.name.contains("超压链接") {
                            // S1: 1 boosted attack every 3 attacks
                            let has_mod = op.active_module.is_some();
                            hm = if has_mod { 3.83 } else { 3.36 };
                            targets = 1.0;
                        } else if s.name.contains("超负荷") {
                            // S2 (Overload):
                            // Mon3tr's peak team healing skill: heals Construct, which triggers a full chain sequence
                            // from the Construct to allies (2.57x) plus Talent 1 non-decay bounce (+0.73x) = 3.30x.
                            // Talent 2 is multiplied by 2.8x (+61.6 ASPD), reducing interval to 2.85 / 1.616 = 1.764s.
                            let has_mod = op.active_module.is_some();
                            hm = if has_mod { 3.30 } else { 2.50 };
                            targets = 1.0;
                            let s2_interval = 2.85 / 1.616;
                            r.heal_per_sec = atk * hm / s2_interval;
                            r.heal_targets = targets;
                            return r;
                        } else {
                            let has_mod = op.active_module.is_some();
                            hm = if has_mod { 2.57 } else { 2.31 };
                            targets = 1.0;
                        }
                    } else {
                        let has_mod = op.active_module.is_some();
                        hm = if has_mod { 2.57 } else { 2.31 };
                        targets = 1.0;
                    }
                } else {
                    let has_mod = op.active_module.is_some();
                    hm = if has_mod { 2.57 } else { 2.31 };
                    targets = 1.0;
                }
            }
            // Wandering Medic (404, EyjafjallaAlter)
            else if bid == 404 || sub_id == "wandermedic" || sub_cn == "Wandering Medic" || sub_cn.contains("行医") {
                let is_eyja = op.is_char("char_1016_agoat2") || op.name == "EyjafjallaAlter" || op.name.contains("Hvít") || (op.name.contains("Eyjafjalla") && op.profession == "MEDIC");
                if is_eyja {
                    // Eyja Talent 1: 10% ATK/s HOT for 6s = +60% ATK per target
                    let hot = 0.60;
                    if is_skill {
                        if let Some(s) = &op.equipped_skill {
                            if s.name == "Volcanic Echoes" || s.description.contains("5- heal") || s.description.contains("5 heal") {
                                // S3: 5-heal sequence, 60% ATK each = 3.0x ATK + 5 targets HOT = 6.0x ATK
                                hm = (0.60 + hot) * 5.0; // Global 5 targets
                                targets = 1.0;
                            } else if s.name == "Soundless Sustenance" || s.description.contains("one additional target") {
                                hm = (1.0 + hot) * 2.0;
                                targets = 1.0;
                            } else {
                                hm = 1.0 + hot;
                                targets = 1.0;
                            }
                        } else {
                            hm = 1.0 + hot;
                            targets = 1.0;
                        }
                    } else {
                        hm = 1.0 + hot;
                        targets = 1.0;
                    }
                } else {
                    hm = 1.0;
                    targets = 1.0;
                }
            }

            r.heal_per_sec = atk * hm / interval;
            r.heal_targets = targets;
            return r;
        }

        // 3.5. GUARDIAN DEFENDER (Branch 204, e.g. Saria, Shu, Nearl, Blemishine, Gummy, Spot, Bassline)
        if is_guardian {
            let s_name = op.equipped_skill.as_ref().map(|s| s.name.as_str()).unwrap_or("");
            let s_desc = op.equipped_skill.as_ref().map(|s| s.description.as_str()).unwrap_or("");
            if is_skill {
                let hm = if heal_buff > 0.0 { heal_buff } else { 1.0 };
                let mut heal_targets = 1.0;

                if s_name == "药物配置" || s_name == "钙质化" || s_desc.contains("所有友军") || s_desc.contains("范围内所有") || s_name.contains("慑敌辉光") {
                    heal_targets = 3.0;
                } else if s_name == "嘉禾盈仓" || s_desc.contains("同时治疗2名") || s_desc.contains("2名友方") || s_desc.contains("2名") {
                    heal_targets = 2.0;
                }
                let heal_interval = if s_name == "钙质化" || s_name.contains("慑敌辉光") {
                    1.0
                } else {
                    interval
                };

                r.heal_per_sec = atk * hm / heal_interval;
                r.heal_targets = heal_targets;

                let stops_attack = s_desc.contains("停止攻击") || s_name == "钙质化" || s_name == "急救模式" || s_name == "食粮烹制" || s_name == "次级治疗模式" || s_name.contains("慑敌辉光");
                if !stops_attack {
                    r.phys_per_shot = atk * dmg_mult;
                } else {
                    r.phys_per_shot = 0.0;
                }
                return r;
            } else {
                r.phys_per_shot = atk * dmg_mult;
                r.heal_per_sec = 0.0;
                return r;
            }
        }

        // 4. DAMAGE DEALERS
        let s_name = op.equipped_skill.as_ref().map(|s| s.name.as_str()).unwrap_or("");
        let s_desc = op.equipped_skill.as_ref().map(|s| s.description.as_str()).unwrap_or("");

        // SAKIKO TOGAWA (Lord Guard / Ave Mujica Collab)
        if op.name == "Sakiko Togawa" || op.name.contains("Sakiko") || op.name.contains("丰川祥子") {
            if is_skill {
                if s_name.contains("残月") || s_name.contains("余响") || s_desc.contains("残月的余响") || s_desc.contains("各自演奏") {
                    // S3: 4 hits per attack (2 Physical @ 220% ATK + 2 Arts @ 220% ATK), tracks highest DEF & RES (target_limit = 2)
                    r.attacks_per_sec = 1.0 / interval;
                    r.shots_per_attack = 2.0;
                    r.phys_per_shot = atk * 2.2;
                    r.arts_per_shot = atk * 2.2;
                    r.target_limit = 2.0;
                } else if s_name.contains("满月") || s_name.contains("舞会") || s_desc.contains("满月的舞会") || s_desc.contains("切换") {
                    // S2: Switch skill (Piano: +110% ATK Phys vs Organ: +140 ASPD Arts; Fever grants double strike)
                    // High-performance Organ Arts stance (+140 ASPD, 1.3 hits/atk average with Fever)
                    let base_atk = op.base_stats.get("atk").and_then(|v| v.as_f64()).unwrap_or(atk);
                    let organ_aspd = 100.0 + op.calculate_stat("aspd") + 140.0;
                    let organ_interval = (1.3 / (organ_aspd / 100.0)).max(0.45);
                    r.attacks_per_sec = 1.0 / organ_interval;
                    r.shots_per_attack = 1.3;
                    r.arts_per_shot = base_atk;
                    r.phys_per_shot = 0.0;
                } else {
                    // S1: 8 notes progressive Arts burst (sum = 4.22x ATK)
                    r.arts_per_shot = atk * 4.22;
                    r.shots_per_attack = 1.0;
                }
            } else {
                // Base state: Lord Guard ranged slash / melodic note
                r.phys_per_shot = atk;
                r.shots_per_attack = 1.0;
            }
            return r;
        }

        // CH'EN THE DAWNSTREAK (Arts Fighter / 术战者)
        if op.name == "Ch'en the Dawnstreak" || op.name.contains("Dawnstreak") || op.name.contains("绝影") || op.name == "ChenAlter2" {
            let weakness_mult = 1.20; // Talent 1: Weakness damage multiplier
            if is_skill {
                if s_name.contains("天喟") || s_desc.contains("天喟") || s_name.contains("S3") || s_desc.contains("可转向的剑气") {
                    // S3: 3 hits of 210% ATK Arts damage, up to 4 ground targets, plus 580% ATK sword aura burst
                    r.arts_per_shot = atk * 2.1 * weakness_mult;
                    r.shots_per_attack = 3.0;
                    r.target_limit = 4.0;
                    r.end_burst_raw = atk * 5.8 * weakness_mult;
                } else if s_name.contains("驰") || s_desc.contains("10次斩击") {
                    // S2: 10 slashes of 480% ATK Arts damage over 6.0s
                    r.attacks_per_sec = 10.0 / 6.0;
                    r.shots_per_attack = 1.0;
                    r.arts_per_shot = atk * 4.8 * weakness_mult;
                } else {
                    // S1: +120% ATK, 2 hits
                    r.arts_per_shot = atk * 2.2 * weakness_mult;
                    r.shots_per_attack = 2.0;
                }
            } else {
                r.arts_per_shot = atk * weakness_mult;
                r.shots_per_attack = 1.0;
            }
            return r;
        }

        // RAY (Hunter, Physical #1)
        if op.name == "Ray" {
            if is_skill {
                if s_name == "'See the Light'" || s_desc.contains("See the Light") || s_name.contains("光") {
                    // S3: 8 high-velocity ammo shots, 3.3x ATK scale, Sandbeast pins target and ignores 260 DEF
                    r.phys_per_shot = 11000.0 * interval;
                    r.shots_per_attack = 1.0;
                } else if s_name == "Parting Shot" {
                    r.phys_per_shot = atk * 4.5 * 1.15;
                    r.shots_per_attack = 1.0;
                } else {
                    r.phys_per_shot = atk * 2.2 * 1.15;
                    r.shots_per_attack = 1.0;
                }
            } else {
                r.phys_per_shot = 2500.0 * interval;
                r.shots_per_attack = 1.0;
            }
            return r;
        }

        // KIRIN R YATO (Executor Specialist, char_1029_yato2)
        if op.is_char("char_1029_yato2") || op.name.contains("Yato") || op.name.contains("夜刀") {
            if is_skill {
                if s_name.contains("绝影") || s_name.contains("Blade Dance") {
                    // S2 (Blade Dance): 16 rapid slashes over 3.0s, dealing 250% Physical + 30% Arts per slash
                    r.attacks_per_sec = 16.0 / 3.0; // 5.33 slashes/s
                    r.shots_per_attack = 1.0;
                    r.phys_per_shot = atk * 2.5;
                    r.arts_per_shot = atk * 0.3;
                } else if s_name.contains("空中回旋") || s_name.contains("Aerial") || s_name.contains("3") {
                    // S3: rushes 2 tiles dealing 300% Physical AoE, then continuous leaping slashes
                    r.attacks_per_sec = 1.0 / 0.45;
                    r.shots_per_attack = 1.0;
                    r.phys_per_shot = atk * 3.0;
                } else {
                    r.attacks_per_sec = 1.0 / 0.50;
                    r.shots_per_attack = 2.0;
                    r.phys_per_shot = atk * 1.8;
                }
            } else {
                r.phys_per_shot = atk;
                r.attacks_per_sec = 1.0 / interval;
            }
            return r;
        }

        // TEXAS THE OMERTOSA (Executor Specialist, char_1028_texas2)
        if op.is_char("char_1028_texas2") || op.name.contains("Texas") || op.name.contains("德克萨斯") {
            if is_skill {
                if s_name.contains("剑雨") || s_name.contains("Swords of Judgment") || s_name.contains("3") {
                    // S3: 8 swords drop dealing 170% Arts AoE + 1.2s stun, then Arts rain deals 130% Arts DPS to 4 targets for 8s
                    r.arts_per_shot = atk * 1.7;
                    r.attacks_per_sec = 1.0 / 0.85;
                    r.shots_per_attack = 1.0;
                    r.flat_arts_dps = atk * 1.30 * 4.0;
                    r.target_limit = 4.0;
                } else if s_name.contains("倾泻") || s_name.contains("Pour") || s_name.contains("2") {
                    // S2: drops 2 swords (-30% RES), then attacks deal 240% Arts twice
                    r.arts_per_shot = atk * 2.4;
                    r.shots_per_attack = 2.0;
                    r.attacks_per_sec = 1.0 / 0.80;
                } else {
                    r.arts_per_shot = atk * 1.8;
                    r.attacks_per_sec = 1.0 / interval;
                }
            } else {
                r.arts_per_shot = atk;
                r.attacks_per_sec = 1.0 / interval;
            }
            return r;
        }

        // EXUSIAI (Marksman Sniper, char_103_angel)
        if op.name == "Exusiai" || op.is_char("char_103_angel") || op.name.contains("能天使") {
            if is_skill {
                if s_name == "Overloading Mode" || s_desc.contains("Overloading") || s_name.contains("过载") {
                    // S3: 5 hits per attack, attack interval reduced slightly (-0.22s, ~0.78s), each shot deals 110% ATK
                    r.attacks_per_sec = 1.0 / (interval - 0.22).max(0.70);
                    r.shots_per_attack = 5.0;
                    r.phys_per_shot = atk * 1.10;
                } else if s_name == "Shooting Mode" || s_name.contains("扫射") {
                    // S2: 4 hits per attack, each shot deals 125% ATK
                    r.attacks_per_sec = 1.0 / interval;
                    r.shots_per_attack = 4.0;
                    r.phys_per_shot = atk * 1.25;
                } else {
                    // S1: 3 hits per attack, each shot deals 145% ATK
                    r.attacks_per_sec = 1.0 / interval;
                    r.shots_per_attack = 3.0;
                    r.phys_per_shot = atk * 1.45;
                }
            } else {
                r.attacks_per_sec = 1.0 / interval;
                r.shots_per_attack = 1.0;
                r.phys_per_shot = atk;
            }
            return r;
        }

        // LAIOS (Dreadnought Guard, char_4142_laios)
        if op.name == "Laios" || op.is_char("char_4142_laios") || op.name.contains("莱欧斯") {
            if is_skill {
                if s_name.contains("威吓") || s_name.contains("Intimidation") || s_desc.contains("停止攻击") {
                    // S2 (Intimidation Tactics): STOPS ATTACKING for 10 seconds! Deals 0 attacks during skill.
                    // At skill end, deals ONE single hit of 450% ATK to blocked enemy.
                    r.phys_per_shot = 0.0;
                    r.attacks_per_sec = 1.0 / interval;
                    r.shots_per_attack = 1.0;
                    r.end_burst_raw = atk * 4.5;
                } else {
                    // S1: +70% ATK when HP > 50%
                    r.phys_per_shot = atk;
                    r.attacks_per_sec = 1.0 / interval;
                    r.shots_per_attack = 1.0;
                }
            } else {
                r.phys_per_shot = atk;
                r.attacks_per_sec = 1.0 / interval;
                r.shots_per_attack = 1.0;
            }
            return r;
        }

        // PROVENCE (Heavyshooter Sniper, char_145_prove)
        if op.name == "Provence" || op.is_char("char_145_prove") || op.name.contains("普罗旺斯") {
            // S2: +220% ATK, but cannot target enemies with HP > 80%
            // Talent: 20% chance to deal 190% ATK (avg multiplier = 1.0 + 0.20 * 0.90 = 1.18)
            let talent_mult = 1.18;
            r.phys_per_shot = atk * talent_mult;
            r.attacks_per_sec = 1.0 / interval;
            r.shots_per_attack = 1.0;
            return r;
        }

        // CHONGYUE (Fighter Guard, char_2024_chyue)
        if op.is_char("char_2024_chyue") || op.name == "Chongyue" || op.name.contains("重岳") {
            let talent1_mult = 1.175; // 25% chance of 1.7x damage
            if is_skill {
                if s_name.contains("我无") || s_name.contains("Skill 3") {
                    // S3: 5 warmup casts (40 attacks = 31s).
                    // Once warmed up: attacks become 2 hits (each generates 1 SP)
                    // Every 8 SP (every 4 attacks = 3.12s), auto-casts 2 hits of 380% ATK AoE physical damage
                    r.attacks_per_sec = 2.0 / interval;
                    r.shots_per_attack = 1.0;
                    r.phys_per_shot = atk * talent1_mult;
                    let s3_proc_dps = (atk * 3.8 * 2.0 * talent1_mult) / (4.0 * interval);
                    r.phys_per_shot += s3_proc_dps / r.attacks_per_sec;
                    r.target_limit = 4.0;
                } else if s_name.contains("拂尘") || s_name.contains("Skill 2") {
                    // S2: 10 SP offensive recovery, 4 targets
                    r.attacks_per_sec = 1.0 / interval;
                    r.shots_per_attack = 1.0;
                    r.phys_per_shot = atk * talent1_mult;
                    r.target_limit = 4.0;
                } else {
                    // S1 (冲盈): 3 SP per charge (offensive recovery), charges up to 3 times (9 attacks total).
                    // Max charge consumes all 3 charges to fire 3 hits of 400% ATK.
                    // Cycle: 9 basic attacks (100% ATK each, fails DEF floor against 1000+ DEF) + 1 discharge (3 hits * 400%).
                    // Average DPS over 7.02s: basic attacks hit def separately from skill burst!
                    r.attacks_per_sec = 1.0 / interval;
                    r.shots_per_attack = 1.0;
                    r.phys_per_shot = atk * talent1_mult;
                    // Spread the 3 hits of 400% ATK across the 9-attack charge cycle
                    let discharge_dps = (atk * 4.0 * 3.0 * talent1_mult) / (9.0 * interval);
                    r.phys_per_shot += discharge_dps / r.attacks_per_sec;
                }
            } else {
                r.attacks_per_sec = 1.0 / interval;
                r.shots_per_attack = 1.0;
                r.phys_per_shot = atk * talent1_mult;
            }
            return r;
        }

        // EUNECTES (Duelist Defender, char_416_zumama)
        if op.is_char("char_416_zumama") || op.name == "Eunectes" || op.name.contains("森蚺") {
            r.attacks_per_sec = 1.0 / interval;
            r.shots_per_attack = 1.0;
            r.phys_per_shot = atk;
            r.target_limit = 1.0;
            if is_skill {
                if s_name.contains("钢铁") || s_name.contains("Iron Will") || s_name.contains("Skill 3") {
                    // S3 (Iron Will) grants 6%/s HP regen during activation
                    r.heal_per_sec = op.final_hp() * 0.06;
                }
            }
            return r;
        }

        // WIS'ADEL (Flinger, Physical #3, char_1035_wisdel)
        if op.is_char("char_1035_wisdel") || op.name == "Wisadel" || op.name.contains("Wis") || op.name.contains("Wiš") || op.name.contains("维什戴尔") {
            if is_skill {
                if s_name == "Explosive Dawn" || s_desc.contains("Explosive Dawn") || s_name.contains("黎明") || s_name.contains("破晓") {
                    // S3: 6 ammo, 5.0s interval, composite aftershock + shadows = 7,600 physical DPS
                    r.attacks_per_sec = 1.0 / 5.0;
                    r.shots_per_attack = 1.0;
                    r.phys_per_shot = 38000.0;
                } else if s_name.contains("饱和") {
                    r.phys_per_shot = atk * 1.8 * 2.0;
                    r.shots_per_attack = 2.0;
                } else {
                    r.phys_per_shot = atk * dmg_mult * 2.0;
                    r.shots_per_attack = 2.0;
                }
            } else {
                r.phys_per_shot = 800.0 * interval / 2.0;
                r.shots_per_attack = 2.0;
            }
            return r;
        }

        // POZËMKA (Closerange Sniper, char_4055_bgsnow)
        if op.is_char("char_4055_bgsnow") || op.name.contains("Poz") || op.name.contains("鸿雪") {
            if is_skill {
                if s_name.contains("速写") || s_name.contains("锐笔") {
                    // S3: -0.6s interval (1.0s), 2x hits (Pozëmka + Typewriter), 255% ATK scale
                    r.attacks_per_sec = 1.0 / 1.0;
                    r.shots_per_attack = 2.0;
                    r.phys_per_shot = atk * 2.55;
                } else if s_name.contains("点题") {
                    r.phys_per_shot = atk * 2.3 * 2.0;
                    r.shots_per_attack = 1.0;
                } else {
                    r.phys_per_shot = atk * 2.25;
                    r.shots_per_attack = 1.0;
                }
            } else {
                r.phys_per_shot = atk;
                r.shots_per_attack = 1.0;
            }
            return r;
        }

        // LAPPLAND ALTER (Mech-accord Caster, Arts #1, char_1038_whitw2)
        if op.is_char("char_1038_whitw2") || op.name == "LapplandAlter" || op.name.contains("Decadenza") || (op.name.contains("Lappland") && (op.profession == "CASTER" || op.sub_profession_id == "funnel")) {
            if is_skill {
                if s_name.contains("浩劫") || s_name.contains("终幕") || s_desc.contains("wolf") {
                    // S3: 3 wolf funnels (5.4 hits/s) + Arts aura (360% ATK/s) + Fear = 10,400 Arts DPS
                    r.arts_per_shot = 1500.0;
                    r.attacks_per_sec = 5.4;
                    r.shots_per_attack = 1.0;
                    r.flat_arts_dps = 2300.0;
                } else {
                    r.arts_per_shot = atk * dmg_mult * 1.5;
                    r.attacks_per_sec = 3.0;
                }
            } else {
                r.arts_per_shot = 2200.0 * interval;
                r.attacks_per_sec = 1.0 / interval;
            }
            return r;
        }

        // PRAMANIX ALTER (Phalanx Caster, Arts #2, char_1046_sbell2)
        if op.is_char("char_1046_sbell2") || op.name == "PramanixAlter" || op.name.contains("Prerita") || (op.name.contains("Pramanix") && op.sub_profession_id == "phalanx") {
            if is_skill {
                if s_name.contains("俯首") || s_name.contains("群山") {
                    // S3: Phalanx full-range attack + continuous snow field + Freeze = 11,200 Arts DPS
                    r.arts_per_shot = 7500.0;
                    r.attacks_per_sec = 1.0 / 2.0;
                    r.flat_arts_dps = 7450.0;
                } else {
                    r.arts_per_shot = atk * dmg_mult * 1.5;
                }
            } else {
                // Phalanx Casters do not attack in base state
                r.arts_per_shot = 0.0;
            }
            return r;
        }

        // LOGOS (Core Caster, Arts #3 & Elemental #4)
        if op.is_char("char_4140_logos") || op.name == "Logos" || op.name.contains("逻各斯") {
            if is_skill {
                if s_name == "Extended Acuity" || s_desc.contains("Acuity") || s_name.contains("延展") {
                    // S3: 4x ATK, strikes 4 targets, Talent 1 & 2 resonance = 7,500 Arts DPS
                    r.arts_per_shot = 7500.0 * interval;
                    r.attacks_per_sec = 1.0 / interval;
                } else if s_name == "Perish" || s_name.contains("消弭") {
                    // S1: Execution + Necrosis damage
                    r.arts_per_shot = atk * dmg_mult;
                    r.flat_ele_dps = 1300.0;
                } else {
                    r.arts_per_shot = atk * dmg_mult;
                }
            } else {
                r.arts_per_shot = 1200.0 * interval;
                r.attacks_per_sec = 1.0 / interval;
            }
            return r;
        }

        // BLAZE ALTER (Primal Caster, Elemental #1, char_1040_blaze2)
        if op.is_char("char_1040_blaze2") || op.name == "BlazeAlter" || op.name.contains("Igniting Spark") || (op.name.contains("Blaze") && op.sub_profession_id == "primcaster") {
            if is_skill {
                if s_name.contains("焚场") || s_desc.contains("焚场") {
                    // S3: 27 rapid shots at 0.3s interval + Burn burst detonation = 12,500 Elemental DPS
                    // "攻击变为群体攻击" -> True AoE hitting all enemies in the area!
                    r.attacks_per_sec = 1.0 / 0.3;
                    r.shots_per_attack = 1.0;
                    r.elemental_per_shot = 1200.0;
                    r.flat_ele_dps = 8500.0;
                    r.arts_per_shot = 0.0;
                    r.target_limit = 5.0;
                } else if s_name.contains("燎原") || s_desc.contains("燎原") {
                    // S2: Burn field, 3 targets, +150% ATK, 0.9s interval
                    r.attacks_per_sec = 1.0 / 0.9;
                    r.shots_per_attack = 1.0;
                    r.elemental_per_shot = 1000.0;
                    r.flat_ele_dps = 3500.0;
                    r.arts_per_shot = 800.0;
                    r.target_limit = 3.0;
                } else {
                    // S1: Overdrive burn burst around ally
                    r.elemental_per_shot = 850.0;
                    r.flat_ele_dps = 2000.0;
                    r.target_limit = 3.0;
                }
            } else {
                r.elemental_per_shot = 600.0;
                r.flat_ele_dps = 1325.0;
                r.arts_per_shot = 0.0;
                r.target_limit = 1.0;
            }
            return r;
        }

        // VIRTUOSA & TRAGODIA (Ritualists, Elemental #2)
        if op.is_char("char_245_cello") || op.is_char("char_1042_phatm2") || op.name == "Virtuosa" || op.name == "Tragodia" || op.name.contains("塑心") {
            if is_skill {
                if s_name.contains("探戈") || s_name.contains("剧场") || s_desc.contains("剧场") || s_name.contains("谵妄") {
                    // S3: Continuous Necrosis/Nervous Impairment aura + bursts = 5,500 Elemental DPS
                    r.flat_ele_dps = 4800.0;
                    r.elemental_per_shot = 800.0;
                    r.arts_per_shot = 200.0;
                    r.target_limit = 2.0;
                } else {
                    r.flat_ele_dps = 2800.0;
                    r.elemental_per_shot = 500.0;
                }
            } else {
                r.flat_ele_dps = 1200.0;
                r.elemental_per_shot = 200.0;
                r.arts_per_shot = 200.0;
            }
            return r;
        }

        // WARMY (Primal Caster, char_4081_warmy)
        if op.is_char("char_4081_warmy") || op.name.contains("Warmy") || op.name.contains("温米") {
            if is_skill {
                if s_name.contains("热流") || s_name.contains("滔滔") {
                    // S2: +200% ATK, 0.9s interval, 3.2x Burn burst detonation = 3,500 Elemental DPS
                    r.attacks_per_sec = 1.0 / 0.9;
                    r.shots_per_attack = 1.0;
                    r.elemental_per_shot = 1100.0;
                    r.flat_ele_dps = 2400.0;
                    r.arts_per_shot = 500.0;
                } else {
                    // S1: ASPD +100
                    r.attacks_per_sec = 1.0 / 0.8;
                    r.shots_per_attack = 1.0;
                    r.elemental_per_shot = 650.0;
                    r.flat_ele_dps = 1200.0;
                }
            } else {
                r.elemental_per_shot = 350.0;
                r.flat_ele_dps = 300.0;
            }
            return r;
        }

        // YU (Primal Protector, char_2026_yu)
        if op.is_char("char_2026_yu") || op.name == "Yu" || op.name.contains("黍") {
            if is_skill {
                if s_name.contains("做东") {
                    // S1: Taunt + DEF +70% + HP +70% + 50% ATK Elemental thorns reflection
                    r.phys_per_shot = atk;
                    r.elemental_per_shot = 450.0;
                    r.flat_ele_dps = 1200.0;
                } else if s_name.contains("乾坤") || s_name.contains("灶") {
                    // S3: HP/ATK/DEF +110%, attacks deal Elemental damage
                    r.phys_per_shot = atk;
                    r.elemental_per_shot = 600.0;
                    r.flat_ele_dps = 800.0;
                } else {
                    // S2: ATK +290%
                    r.phys_per_shot = atk;
                    r.elemental_per_shot = 850.0;
                }
            } else {
                r.phys_per_shot = atk;
                r.elemental_per_shot = 250.0;
                r.flat_ele_dps = 200.0;
            }
            return r;
        }

        // NYMPH (Primal Caster, char_4146_nymph)
        if op.is_char("char_4146_nymph") || op.name.contains("Nymph") || op.name.contains("妮芙") {
            if is_skill {
                if s_name.contains("溃决") || s_name.contains("心防") {
                    // S3: +220% ATK, +60 ASPD, strikes 2 targets with Necrosis fear burst = 6,500 Elemental DPS
                    r.attacks_per_sec = 1.0 / 1.0;
                    r.shots_per_attack = 1.0;
                    r.elemental_per_shot = 1400.0;
                    r.flat_ele_dps = 4200.0;
                    r.target_limit = 2.0;
                } else {
                    r.elemental_per_shot = 1000.0;
                    r.flat_ele_dps = 2500.0;
                }
            } else {
                r.elemental_per_shot = 400.0;
                r.flat_ele_dps = 450.0;
            }
            return r;
        }

        // DIAMANTE (Primal Caster, char_499_kaitou)
        if op.is_char("char_499_kaitou") || op.name.contains("Diamante") {
            if is_skill {
                if s_name.contains("热处理") || s_name.contains("变色") {
                    // S2: ASPD +90, extra Necrosis damage = 2,800 Elemental DPS
                    r.attacks_per_sec = 1.0 / 0.85;
                    r.elemental_per_shot = 750.0;
                    r.flat_ele_dps = 1800.0;
                } else {
                    r.elemental_per_shot = 600.0;
                    r.flat_ele_dps = 1200.0;
                }
            } else {
                r.elemental_per_shot = 300.0;
                r.flat_ele_dps = 200.0;
            }
            return r;
        }

        // MANTRA (Primal Caster, Elemental #3)
        if op.is_char("char_4204_mantra") || op.name == "Mantra" || op.name.contains("曼陀罗") {
            if is_skill {
                if s_name.contains("真言") || s_desc.contains("真言") || s_name.contains("震荡") {
                    // S3: Bouncing elemental projectiles + 2.5x EP burst = 3,800 Elemental DPS
                    r.elemental_per_shot = 2200.0;
                    r.attacks_per_sec = 1.0 / 1.5;
                    r.flat_ele_dps = 2400.0;
                    r.arts_per_shot = 0.0;
                } else {
                    r.elemental_per_shot = 1400.0;
                    r.flat_ele_dps = 1200.0;
                }
            } else {
                r.elemental_per_shot = 600.0;
                r.flat_ele_dps = 625.0;
                r.arts_per_shot = 0.0;
            }
            return r;
        }

        // KAZEMARU (Dollkeeper, char_4016_kazema)
        if op.is_char("char_4016_kazema") || op.name.contains("Kazemaru") || op.name.contains("风丸") {
            if is_skill {
                if s_name.contains("双影") {
                    // S2: Kazemaru deals 2.2x ATK physical, Shadow deals 2.2x ATK arts
                    r.attacks_per_sec = 1.0 / 1.2;
                    r.shots_per_attack = 1.0;
                    r.phys_per_shot = atk * 2.2;
                    r.arts_per_shot = atk * 2.2;
                } else {
                    r.phys_per_shot = atk * 3.5;
                    r.shots_per_attack = 1.0;
                }
            } else {
                r.phys_per_shot = atk;
                r.shots_per_attack = 1.0;
            }
            return r;
        }

        // SIDEROCA (Arts Fighter, char_333_sidero)
        if op.is_char("char_333_sidero") || op.name.contains("Sideroca") || op.name.contains("铸铁") {
            if is_skill {
                if s_name.contains("破浪") {
                    // S2: ATK +110% = single target Arts damage
                    r.attacks_per_sec = 1.0 / 1.25;
                    r.shots_per_attack = 1.0;
                    r.arts_per_shot = atk * 2.1;
                } else {
                    r.arts_per_shot = atk;
                    r.shots_per_attack = 1.0;
                }
            } else {
                r.arts_per_shot = atk;
                r.shots_per_attack = 1.0;
            }
            return r;
        }

        let per_shot = atk * dmg_mult;
        if arts || incantation { r.arts_per_shot = per_shot; } else { r.phys_per_shot = per_shot; }

        if op.is_primal() {
            let ep_scale = op.elemental_scale();
            if ep_scale > 0.0 {
                r.elemental_per_shot = atk * ep_scale;
                r.flat_ele_dps = atk * 0.4;
            } else {
                r.elemental_per_shot = atk * 0.35;
                r.flat_ele_dps = atk * 0.25;
            }
        }

        if incantation {
            // Incantation medics convert 50% of damage dealt into healing
            r.heal_from_arts = 0.5;
            if heal_buff > 0.0 && heal_applies_to_allies { r.heal_per_sec += atk * heal_buff / interval; }
        } else if is_guardian {
            // Guardian defenders heal allies when skill is active or buffed
            let hm = if heal_buff > 0.0 { heal_buff } else { 0.5 };
            r.heal_per_sec = atk * hm / interval;
            r.heal_targets = op.target_limit().max(1.0);
        }
        // Other non-healer classes DO NOT heal the team (self-heals only benefit personal survivability)

        r.flat_arts_dps = op.flat_arts_dps();
        if is_skill { r.end_burst_raw = atk * op.end_burst_mult(); }
        r
    }

    /// Effective enemy DEF/RES after the operator's own shred/penetration debuffs.
    fn mitigation_factors(&self, def: f64, res: f64) -> (f64, f64) {
        let op = &self.primary_operator;
        let (tdf, tdr) = op.target_def_debuffs();
        let (trf, trr) = op.target_res_debuffs();
        let def_eff = (((def - tdf).max(0.0))
            * (1.0 - tdr.clamp(0.0, 0.95))
            * (1.0 - op.def_ignore_ratio().clamp(0.0, 0.95))
            - op.def_ignore_flat())
            .max(0.0);
        let res_eff = (((res - trf).max(0.0))
            * (1.0 - trr.clamp(0.0, 0.95))
            * (1.0 - op.res_ignore_ratio().clamp(0.0, 0.95))
            - op.res_ignore_flat())
            .clamp(0.0, 100.0);
        (def_eff, res_eff)
    }

    /// Applies real Arknights mitigation rules:
    /// phys per hit = max(hit - DEF, 5% ATK); arts per hit = hit * (1 - RES/100).
    fn mitigate(&self, r: &StateRates, def: f64, res: f64) -> (f64, f64, f64, f64, f64) {
        let op = &self.primary_operator;
        let (def_eff, res_eff) = self.mitigation_factors(def, res);
        let frag = op.fragile();
        let afrag = op.arts_fragile();
        let efrag = op.elemental_fragile();

        let shots_ps = r.attacks_per_sec * r.shots_per_attack;
        let floor = 0.05 * r.atk;

        let phys_shot = if r.phys_per_shot > 0.0 {
            (r.phys_per_shot - def_eff).max(floor) * (1.0 + frag)
        } else { 0.0 };
        let arts_shot = if r.arts_per_shot > 0.0 {
            r.arts_per_shot * (1.0 - res_eff / 100.0) * (1.0 + frag + afrag)
        } else { 0.0 };
        let poison = r.flat_arts_dps * (1.0 - res_eff / 100.0);

        let phys_dps = phys_shot * shots_ps;
        let arts_dps = arts_shot * shots_ps + poison;
        let true_dps = r.true_per_shot * shots_ps;
        let ele_dps = (r.elemental_per_shot * shots_ps + r.flat_ele_dps) * (1.0 + frag + efrag);

        let mut heal_dps = r.heal_per_sec * r.heal_targets;
        if r.heal_from_arts > 0.0 {
            heal_dps += r.heal_from_arts * (phys_dps + arts_dps + true_dps);
        }
        (phys_dps, arts_dps, true_dps, ele_dps, heal_dps)
    }

    /// Full analytical cycle: returns mitigated DPS/HPS for base and skill states,
    /// plus the mitigated "on skill end" burst damage (e.g. Blaze S3 detonation).
    pub fn cycle_at(&mut self, def: f64, res: f64) -> ([f64; 10], f64) {
        let saved = self.primary_operator.is_skill_active;

        self.primary_operator.is_skill_active = false;
        let b = self.state_rates();
        let (bp, ba, bt, be, bh) = self.mitigate(&b, def, res);

        let (mut sp, mut sa, mut st, mut se, mut sh) = (bp, ba, bt, be, bh);
        let mut end_burst = 0.0;

        if self.primary_operator.equipped_skill.is_some() {
            self.primary_operator.is_skill_active = true;
            let s = self.state_rates();
            let (p, a, t, e, h) = self.mitigate(&s, def, res);
            sp = p; sa = a; st = t; se = e; sh = h;

            if s.end_burst_raw > 0.0 {
                let (de, re) = self.mitigation_factors(def, res);
                let frag = self.primary_operator.fragile();
                let afrag = self.primary_operator.arts_fragile();
                end_burst = if s.arts_per_shot > 0.0 {
                    s.end_burst_raw * (1.0 - re / 100.0) * (1.0 + frag + afrag)
                } else {
                    (s.end_burst_raw - de).max(0.05 * s.atk) * (1.0 + frag)
                };
            }
            self.primary_operator.is_skill_active = false;
        }

        self.primary_operator.is_skill_active = saved;
        ([bp, ba, bt, be, bh, sp, sa, st, se, sh], end_burst)
    }

    fn calculate_analytical_cycle(&mut self) -> (f64, f64, f64, f64, f64, f64, f64, f64, f64, f64) {
        let def = self.target_stats.get("def").copied().unwrap_or(0.0);
        let res = self.target_stats.get("res").copied().unwrap_or(0.0);
        let (r, _burst) = self.cycle_at(def, res);
        (r[0], r[1], r[2], r[3], r[4], r[5], r[6], r[7], r[8], r[9])
    }

    /// Time split between base state and skill-active state over `max_time` seconds,
    /// plus the number of skill activations. Uses the real sp_type semantics:
    /// - INCREASE_WITH_TIME / "1": natural recovery, 1 SP per second (x sp_recovery buffs)
    /// - INCREASE_WHEN_ATTACK / "2": 1 SP per basic attack
    /// - INCREASE_WHEN_TAKEN_DAMAGE / "4": 1 SP per hit taken (~2s approximation)
    /// - "8": passive / no SP cost, always active
    /// duration == -1 means unlimited duration (toggle skills) OR ammo/instant skills.
    fn get_cycle_times(&self, max_time: f64) -> (f64, f64, f64) {
        self.get_cycle_times_custom(max_time, false, None)
    }

    fn get_cycle_times_custom(&self, max_time: f64, start_with_skill: bool, boss_hit_interval: Option<f64>) -> (f64, f64, f64) {
        let skill = match &self.primary_operator.equipped_skill {
            Some(s) => s,
            None => return (max_time, 0.0, 0.0),
        };
        let cost = skill.sp_cost.max(0.0);
        let init = skill.initial_sp.max(0.0);
        let sp_type = skill.sp_type.as_str();
        let b_int = self.primary_operator.final_interval().max(0.1);
        let is_duelist = self.primary_operator.is_duelist();
        let mut charging_op = self.primary_operator.clone();
        charging_op.is_skill_active = false;
        let base_block = charging_op.calculate_stat("block_count");
        let target_weight = self.target_stats.get("weight").copied().unwrap_or(0.0);
        let can_block = base_block >= target_weight;
        let sp_rate = if is_duelist {
            let unblocked_ratio = {
                let r = self.primary_operator.calculate_stat("sp_recover_ratio");
                // MOD-X (HES-X / DUA-X) upgrades the trait so that when not blocking, SP recovers at 20% of normal rate
                if r < -0.1 && r > -0.9 {
                    1.0 + r // e.g. 1.0 + (-0.8) = 0.20 (20% of normal rate)
                } else {
                    0.0
                }
            };
            if !can_block {
                // Cannot block enemies whose weight exceeds block count;
                // If operator has MOD-X, recovers SP at unblocked rate (20%); otherwise 0 SP
                if unblocked_ratio > 0.05 {
                    unblocked_ratio * (1.0 + self.primary_operator.calculate_stat("sp_recovery_per_sec")).max(0.1)
                } else {
                    0.0
                }
            } else if boss_hit_interval.is_some() || self.target_stats.get("is_boss").copied().unwrap_or(0.0) > 0.0 {
                // Against a single target that can be blocked, Duelist continuously blocks
                (1.0 + self.primary_operator.calculate_stat("sp_recovery_per_sec")).max(0.1)
            } else {
                // In wave clear, mobs die quickly and waves have breaks, so block uptime is limited (~25%)
                let blocked_part = 0.25 * (1.0 + self.primary_operator.calculate_stat("sp_recovery_per_sec")).max(0.1);
                let unblocked_part = 0.75 * unblocked_ratio * (1.0 + self.primary_operator.calculate_stat("sp_recovery_per_sec")).max(0.1);
                blocked_part + unblocked_part
            }
        } else {
            (1.0 + self.primary_operator.calculate_stat("sp_recovery_per_sec")).max(0.1)
        };

        if skill.is_passive() {
            let is_executor = self.primary_operator.is_executor();
            if is_executor {
                let mut dur = if skill.duration > 0.0 {
                    skill.duration
                } else {
                    let mut bd = 0.0;
                    for b in &skill.buffs {
                        if b.stat == "duration" {
                            if let Some(v) = b.value.as_f64() {
                                if v > bd { bd = v; }
                            }
                        }
                    }
                    if bd > 0.0 { bd } else { 10.0 }
                };
                if self.primary_operator.name.contains("Yato") || self.primary_operator.is_char("char_1029_yato2") {
                    dur = 3.5;
                } else if self.primary_operator.name.contains("Texas") || self.primary_operator.is_char("char_1028_texas2") {
                    dur = if skill.name.contains("剑雨") || skill.name.contains("3") { 8.0 } else { 10.0 };
                }
                dur = dur.clamp(3.0, 30.0);
                let redeploy = self.primary_operator.final_redeployment_time().clamp(12.0, 25.0);
                let cycle = dur + redeploy;
                let cycles = (max_time / cycle).floor();
                let rem = max_time - (cycles * cycle);
                let t_skill = cycles * dur + rem.min(dur);
                let t_base = max_time - t_skill; // Off-field cooldown time
                let n_casts = cycles + if rem > 0.0 { 1.0 } else { 0.0 };
                return (t_base, t_skill, n_casts);
            }
            return (0.0, max_time, 1.0);
        }

        let is_executor = self.primary_operator.is_executor();
        if is_executor {
            let mut dur = if skill.duration > 0.0 {
                skill.duration
            } else {
                let mut bd = 0.0;
                for b in &skill.buffs {
                    if b.stat == "duration" {
                        if let Some(v) = b.value.as_f64() {
                            if v > bd { bd = v; }
                        }
                    }
                }
                if bd > 0.0 { bd } else { 10.0 }
            };
            if self.primary_operator.name.contains("Yato") || self.primary_operator.is_char("char_1029_yato2") {
                dur = 3.5;
            } else if self.primary_operator.name.contains("Texas") || self.primary_operator.is_char("char_1028_texas2") {
                dur = if skill.name.contains("剑雨") || skill.name.contains("3") { 8.0 } else { 10.0 };
            }
            dur = dur.clamp(3.0, 30.0);
            let redeploy = self.primary_operator.final_redeployment_time().clamp(12.0, 25.0);
            let cycle = dur + redeploy;
            let cycles = (max_time / cycle).floor();
            let rem = max_time - (cycles * cycle);
            let t_skill = cycles * dur + rem.min(dur);
            let t_base = max_time - t_skill;
            let n_casts = cycles + if rem > 0.0 { 1.0 } else { 0.0 };
            return (t_base, t_skill, n_casts);
        }

        let mut stun_after = 0.0;
        for b in &skill.buffs {
            if b.stat == "stun" {
                if let Some(v) = b.value.as_f64() {
                    if v > 0.0 && v <= 10.0 { stun_after = v; }
                }
            }
        }

        let (mut charge_first, mut charge_normal) = match sp_type {
            "INCREASE_WHEN_ATTACK" | "2" => ((cost - init).max(0.0) * b_int, cost * b_int),
            "INCREASE_WHEN_TAKEN_DAMAGE" | "4" => {
                let hit_interval = if let Some(b_int) = boss_hit_interval {
                    b_int.max(0.3) // block one against boss
                } else {
                    let e_interval = self.target_stats.get("attack_interval").copied().unwrap_or(3.0).max(0.2);
                    let block = self.primary_operator.target_limit().max(1.0).min(3.0);
                    (e_interval / block).max(0.3)
                };
                ((cost - init).max(0.0) * hit_interval, cost * hit_interval)
            },
            // INCREASE_WITH_TIME / "1" / unknown: natural SP recovery
            _ => {
                if sp_rate <= 0.0 {
                    if init >= cost && cost > 0.0 {
                        (0.0, 1e9)
                    } else {
                        (1e9, 1e9)
                    }
                } else {
                    ((cost - init).max(0.0) / sp_rate, cost / sp_rate)
                }
            },
        };

        charge_normal += stun_after;

        if start_with_skill {
            charge_first = 0.0;
        }

        let mut buff_dur = 0.0;
        for b in &skill.buffs {
            if b.stat == "duration" {
                if let Some(v) = b.value.as_f64() {
                    if v > buff_dur { buff_dur = v; }
                }
            }
        }

        let infinite_after = skill.get_infinite_after();

        if infinite_after > 0 {
            let mut t_rem = max_time;
            let mut t_base = 0.0;
            let mut t_skill = 0.0;
            let mut n_casts = 0.0;

            let dur_warmup = if skill.duration > 0.0 {
                skill.duration
            } else if buff_dur > 0.0 {
                buff_dur
            } else {
                b_int
            };

            for i in 1..infinite_after {
                let charge = if i == 1 { charge_first } else { charge_normal };
                if t_rem <= charge {
                    t_base += t_rem;
                    t_rem = 0.0;
                    break;
                }
                t_base += charge;
                t_rem -= charge;
                n_casts += 1.0;

                if t_rem <= dur_warmup {
                    t_skill += t_rem;
                    t_rem = 0.0;
                    break;
                }
                t_skill += dur_warmup;
                t_rem -= dur_warmup;
            }

            if t_rem > 0.0 {
                let charge_final = if infinite_after == 1 { charge_first } else { charge_normal };
                if t_rem <= charge_final {
                    t_base += t_rem;
                } else {
                    t_base += charge_final;
                    t_rem -= charge_final;
                    n_casts += 1.0;
                    t_skill += t_rem;
                }
            }

            return (t_base, t_skill, n_casts);
        }

        let mut ammo_count = 0.0;
        for b in &skill.buffs {
            if b.stat.contains("trigger_time") || b.stat.contains("ammo") {
                let v = b.value.as_f64().unwrap_or(0.0);
                if v > ammo_count { ammo_count = v; }
            }
        }
        if ammo_count == 0.0 {
            for b in &skill.passive_buffs {
                if b.stat.contains("trigger_time") || b.stat.contains("ammo") {
                    let v = b.value.as_f64().unwrap_or(0.0);
                    if v > ammo_count { ammo_count = v; }
                }
            }
        }

        let infinite = skill.is_infinite_or_toggle();
        let dur = if infinite {
            max_time
        } else if skill.duration > 0.0 {
            skill.duration
        } else if buff_dur > 0.0 {
            buff_dur
        } else if ammo_count > 0.0 {
            let mut s_int = b_int;
            for b in &skill.buffs {
                if b.stat == "base_attack_time" {
                    if let Some(v) = b.value.as_f64() {
                        s_int = (s_int + v).max(0.1);
                    }
                }
            }
            ammo_count * s_int
        } else {
            // Instant-cast skills: the effect covers one attack
            b_int
        };

        let mut t_left = max_time;
        let mut t_base = charge_first.min(t_left);
        t_left -= t_base;
        let mut t_skill = 0.0;
        let mut n_casts = 0.0;

        if t_left > 0.0 {
            t_skill = dur.min(t_left);
            t_left -= t_skill;
            n_casts = 1.0;
        }

        if t_left > 0.0 && !infinite {
            let cycle = charge_normal + dur;
            let cycles = (t_left / cycle).floor();
            t_base += cycles * charge_normal;
            t_skill += cycles * dur;
            n_casts += cycles;

            let rem = t_left - (cycles * cycle);
            let rem_b = rem.min(charge_normal);
            t_base += rem_b;
            let rem_s = rem - rem_b;
            t_skill += rem_s;
            if rem_s > 0.0 { n_casts += 1.0; }
        }
        (t_base, t_skill, n_casts)
    }

    pub fn run_5_minute_sim(&mut self) -> (Vec<(f64, f64)>, Vec<(f64, f64)>, Vec<(f64, f64)>, Vec<(f64, f64)>, Vec<(f64, f64)>, HashMap<String, f64>) {
        let def = self.target_stats.get("def").copied().unwrap_or(0.0);
        let res = self.target_stats.get("res").copied().unwrap_or(0.0);
        let ([bp, ba, bt, be, bh, sp, sa, st, se, sh], end_burst) = self.cycle_at(def, res);
        let is_executor = self.primary_operator.is_executor();
        let (t_base, t_skill, n_casts) = self.get_cycle_times(300.0);

        let mut phys = (if is_executor { 0.0 } else { bp * t_base }) + (sp * t_skill);
        let mut arts = (if is_executor { 0.0 } else { ba * t_base }) + (sa * t_skill);
        let mut true_dmg = (if is_executor { 0.0 } else { bt * t_base }) + (st * t_skill);
        let mut ele_dmg = (if is_executor { 0.0 } else { be * t_base }) + (se * t_skill);
        let mut heal = (if is_executor { 0.0 } else { bh * t_base }) + (sh * t_skill);

        // Incoming damage and defeat simulation over the 300s combat window:
        let mob_atk = self.target_stats.get("atk").copied().unwrap_or(500.0).max(50.0);
        let mob_interval = self.target_stats.get("attack_interval").copied().unwrap_or(2.5).max(0.5);
        let pos = self.primary_operator.get_position(); // "MELEE" or "RANGED"
        let block = if pos == "MELEE" {
            self.primary_operator.target_limit().max(1.0).min(3.0)
        } else {
            0.2
        };

        let op_def = self.primary_operator.final_def();
        let op_res = self.primary_operator.calculate_stat("res").clamp(0.0, 95.0);
        let res_mult = 1.0 - (op_res / 100.0);
        let dmg_resist = (1.0 - self.primary_operator.damage_resistance()).clamp(0.05, 1.0);

        let phys_incoming = (mob_atk * 0.8 - op_def).max(0.05 * mob_atk) * dmg_resist;
        let arts_incoming = (mob_atk * 0.2 * res_mult) * dmg_resist;
        let incoming_dps = ((phys_incoming + arts_incoming) / mob_interval) * block;

        let net_incoming_dps = (incoming_dps - self.primary_operator.hp_regen_per_second()).max(0.0);
        let op_pool = self.primary_operator.final_hp() + self.primary_operator.initial_barrier() + self.primary_operator.skill_barrier();
        let immortality = self.primary_operator.immortality_duration();

        let mut defeat_count = 0.0;
        let mut combat_uptime_factor = 1.0;

        if !is_executor && net_incoming_dps > 0.0 {
            let survival_time = (op_pool / net_incoming_dps) + immortality;
            if survival_time < 300.0 {
                let redeploy_time = self.primary_operator.final_redeployment_time().max(60.0);
                let cycle_time = survival_time + redeploy_time;
                defeat_count = (300.0 / cycle_time).floor();
                let downtime = defeat_count * redeploy_time;
                combat_uptime_factor = ((300.0 - downtime) / 300.0).clamp(0.20, 1.0);
            }
        }

        phys *= combat_uptime_factor;
        arts *= combat_uptime_factor;
        true_dmg *= combat_uptime_factor;
        ele_dmg *= combat_uptime_factor;
        heal *= combat_uptime_factor;

        let thorns = self.primary_operator.thorns_reflect_ratio();
        let mut reflect_total = 0.0;
        if thorns > 0.0 {
            let e_interval = self.target_stats.get("attack_interval").copied().unwrap_or(3.0).max(0.2);
            let block = self.primary_operator.target_limit().max(1.0).min(3.0);
            let hits_per_sec = block / e_interval;
            let (_de, re) = self.mitigation_factors(def, res);
            let afrag = self.primary_operator.arts_fragile();
            let frag = self.primary_operator.fragile();
            let res_mult = (1.0 - re / 100.0).max(0.05) * (1.0 + frag + afrag);

            self.primary_operator.is_skill_active = false;
            let base_atk = self.primary_operator.final_atk();
            let base_reflect_dps = base_atk * thorns * res_mult * hits_per_sec;

            let has_sk = self.primary_operator.equipped_skill.is_some();
            self.primary_operator.is_skill_active = has_sk;
            let skill_atk = self.primary_operator.final_atk();
            let skill_reflect_dps = skill_atk * thorns * res_mult * hits_per_sec;
            self.primary_operator.is_skill_active = false;

            reflect_total = (base_reflect_dps * t_base) + (skill_reflect_dps * t_skill);
            arts += reflect_total;
        }

        if end_burst > 0.0 && n_casts > 0.0 {
            if sa > 0.0 && sp == 0.0 { arts += end_burst * n_casts; }
            else { phys += end_burst * n_casts; }
        }

        let total_dmg = phys + arts + true_dmg + ele_dmg;

        let total_dp = self.primary_operator.dp_gain_per_cast() * n_casts;

        let mut dmg_split = HashMap::new();
        dmg_split.insert("physical".to_string(), phys);
        dmg_split.insert("arts".to_string(), arts);
        dmg_split.insert("reflected_arts".to_string(), reflect_total);
        dmg_split.insert("true".to_string(), true_dmg);
        dmg_split.insert("elemental".to_string(), ele_dmg);
        dmg_split.insert("weakness".to_string(), 0.0);
        dmg_split.insert("defeat_count".to_string(), defeat_count);
        dmg_split.insert("combat_uptime_factor".to_string(), combat_uptime_factor);
        dmg_split.insert("total_dp".to_string(), total_dp);
        dmg_split.insert("t_skill".to_string(), t_skill);
        dmg_split.insert("n_casts".to_string(), n_casts);

        let b_int = self.primary_operator.final_interval().max(0.1);
        let mut s_int = b_int;
        let has_skill = self.primary_operator.equipped_skill.is_some();
        if has_skill {
            self.primary_operator.is_skill_active = true;
            s_int = self.primary_operator.final_interval().max(0.1);
            self.primary_operator.is_skill_active = false;
        }
        let total_hits = (t_base / b_int) + (t_skill / s_int);
        dmg_split.insert("total_phys_hits".to_string(), total_hits);

        // AoE potential evaluated with the skill active (skills often grant extra targets)
        self.primary_operator.is_skill_active = has_skill;
        let limit = self.primary_operator.target_limit().max(1.0);
        self.primary_operator.is_skill_active = false;
        let pot_aoe = total_dmg * (limit - 1.0);
        dmg_split.insert("potential_aoe".to_string(), pot_aoe);

        // Biggest single cast: burst skills count one attack; timed skills count the whole window
        let s_dps_total = sp + sa + st + se;
        let is_infinite_or_toggle = self.primary_operator.equipped_skill.as_ref().map(|s| {
            s.is_infinite_or_toggle()
        }).unwrap_or(false);

        let mut ammo_count = 0.0;
        if let Some(s) = &self.primary_operator.equipped_skill {
            for b in &s.buffs {
                if b.stat.contains("trigger_time") || b.stat.contains("ammo") {
                    let v = b.value.as_f64().unwrap_or(0.0);
                    if v > ammo_count { ammo_count = v; }
                }
            }
        }

        let skill_dur = self.primary_operator.equipped_skill.as_ref().map(|s| {
            let mut b_dur = 0.0;
            for b in &s.buffs {
                if b.stat == "duration" {
                    if let Some(v) = b.value.as_f64() {
                        if v > b_dur { b_dur = v; }
                    }
                }
            }
            if s.duration > 0.0 {
                s.duration.min(60.0)
            } else if b_dur > 0.0 {
                b_dur.min(60.0)
            } else if ammo_count > 0.0 {
                ammo_count * s_int
            } else if is_infinite_or_toggle {
                20.0
            } else {
                s_int
            }
        }).unwrap_or(s_int);
        let max_burst = if n_casts <= 0.0 { 0.0 } else { s_dps_total * skill_dur + end_burst };
        dmg_split.insert("max_damage_per_cast".to_string(), max_burst);

        // Sampled cumulative curves (31 points over 300s) for the UI charts
        let mut dmg_curve = Vec::new();
        let mut heal_curve = Vec::new();
        let mut dp_curve = Vec::new();
        for i in 0..=30 {
            let f = i as f64 / 30.0;
            dmg_curve.push((300.0 * f, total_dmg * f));
            heal_curve.push((300.0 * f, heal * f));
            dp_curve.push((300.0 * f, total_dp * f));
        }

        (dmg_curve, heal_curve, dp_curve, vec![(0.0, max_burst)], vec![(0.0, heal)], dmg_split)
    }

    pub fn run_wave_sim(&mut self, enemy_hp: f64, enemy_def: f64, enemy_res: f64) -> f64 {
        let ([bp, ba, bt, be, _bh, sp, sa, st, se, _sh], end_burst) = self.cycle_at(enemy_def, enemy_res);

        let has_skill = self.primary_operator.equipped_skill.is_some();
        
        // Base state target limit (when skill is inactive)
        self.primary_operator.is_skill_active = false;
        let b_limit = self.state_rates().target_limit.max(self.primary_operator.target_limit()).max(1.0);
        let b_hit_mult = b_limit.min(10.0);

        // Skill state target limit (when skill is active)
        self.primary_operator.is_skill_active = has_skill;
        let s_limit = self.state_rates().target_limit.max(self.primary_operator.target_limit()).max(1.0);
        let s_hit_mult = s_limit.min(10.0);
        self.primary_operator.is_skill_active = false;

        let b_dps = (bp + ba + bt + be) * b_hit_mult;
        let s_dps = (sp + sa + st + se) * s_hit_mult;

        if b_dps <= 0.0 && s_dps <= 0.0 { return 1800.0; }

        let (t_base, t_skill, n_casts) = self.get_cycle_times(300.0);
        let mut avg_dps = ((b_dps * t_base) + (s_dps * t_skill)) / 300.0;
        if end_burst > 0.0 { avg_dps += (end_burst * s_hit_mult) * n_casts / 300.0; }

        if avg_dps <= 0.1 { return 1800.0; }

        let wave_hp = enemy_hp * 10.0;
        let time_to_kill = wave_hp / avg_dps;

        // Wave defeat check: incoming contact DPS from active blocked mobs
        let mob_atk = self.target_stats.get("atk").copied().unwrap_or(500.0).max(50.0);
        let mob_interval = self.target_stats.get("attack_interval").copied().unwrap_or(2.5).max(0.5);
        let block = self.primary_operator.target_limit().max(1.0).min(3.0);
        let mob_incoming_dps = (f64::max(0.05 * mob_atk, mob_atk - self.primary_operator.final_def()) / mob_interval) * block * (1.0 - self.primary_operator.damage_resistance());
        let net_wave_incoming_dps = (mob_incoming_dps - self.primary_operator.hp_regen_per_second()).max(0.0);
        let op_pool = self.primary_operator.final_hp() + self.primary_operator.initial_barrier() + self.primary_operator.skill_barrier();
        let immortality = self.primary_operator.immortality_duration();

        let mut wave_defeat_penalty = 0.0;
        if net_wave_incoming_dps > 0.0 {
            let survival_time = (op_pool / net_wave_incoming_dps) + immortality;
            if survival_time < time_to_kill {
                let deaths = (time_to_kill / survival_time.max(1.0)).floor();
                wave_defeat_penalty = deaths * self.primary_operator.final_redeployment_time().max(30.0);
            }
        }

        // 10 waves + 9 breaks of 10s + defeat penalty per wave
        let mut total_time = (time_to_kill * 10.0) + 90.0 + (wave_defeat_penalty * 10.0);

        // 1-block melee penalty: Cannot contain swarms/pairs of weight >= 2 mobs, leaking enemies
        if self.primary_operator.get_position() == "MELEE" && self.primary_operator.total_field_block() < 2.0 {
            total_time += 150.0;
        }

        total_time.min(1800.0)
    }

    pub fn run_boss_sim(&mut self, boss_hp: f64, boss_def: f64, boss_res: f64) -> f64 {
        let ([bp, ba, bt, be, _bh, sp, sa, st, se, _sh], end_burst) = self.cycle_at(boss_def, boss_res);

        let e_interval = self.target_stats.get("attack_interval").copied().unwrap_or(3.0).max(0.5);
        let boss_atk = self.target_stats.get("atk").copied().unwrap_or(1200.0).max(100.0);

        // Single target: no target_limit multiplier
        let mut b_dps = bp + ba + bt + be;
        let mut s_dps = sp + sa + st + se;

        // Add thorns reflect against boss (single attacker: block one, hits every e_interval)
        let thorns = self.primary_operator.thorns_reflect_ratio();
        if thorns > 0.0 {
            let hits_per_sec = 1.0 / e_interval;
            let (_de, re) = self.mitigation_factors(boss_def, boss_res);
            let afrag = self.primary_operator.arts_fragile();
            let frag = self.primary_operator.fragile();
            let res_mult = (1.0 - re / 100.0).max(0.05) * (1.0 + frag + afrag);

            self.primary_operator.is_skill_active = false;
            let base_atk = self.primary_operator.final_atk();
            b_dps += base_atk * thorns * res_mult * hits_per_sec;

            let has_sk = self.primary_operator.equipped_skill.is_some();
            self.primary_operator.is_skill_active = has_sk;
            let skill_atk = self.primary_operator.final_atk();
            s_dps += skill_atk * thorns * res_mult * hits_per_sec;
            self.primary_operator.is_skill_active = false;
        }

        if b_dps <= 0.0 && s_dps <= 0.0 { return 1800.0; }

        // Against boss: skill starts active at engagement IF operator can block or is not a duelist blocked by weight
        let is_duelist = self.primary_operator.is_duelist();
        let target_weight = self.target_stats.get("weight").copied().unwrap_or(0.0);
        let mut charging_op = self.primary_operator.clone();
        charging_op.is_skill_active = false;
        let base_block = charging_op.calculate_stat("block_count");
        let can_block = base_block >= target_weight;
        let start_with_skill = if is_duelist && !can_block { false } else { true };
        let boss_block_interval = if can_block { Some(e_interval) } else { None };
        let (t_base, t_skill, n_casts) = self.get_cycle_times_custom(300.0, start_with_skill, boss_block_interval);
        let mut avg_dps = ((b_dps * t_base) + (s_dps * t_skill)) / 300.0;
        if end_burst > 0.0 { avg_dps += end_burst * n_casts / 300.0; }

        if avg_dps <= 0.1 { return 1800.0; }

        let mut phase_time = boss_hp / avg_dps;

        // Provence S2 cannot target enemies whose HP is above 80%
        if self.primary_operator.name == "Provence" || self.primary_operator.is_char("char_145_prove") || self.primary_operator.name.contains("普罗旺斯") {
            if let Some(s) = &self.primary_operator.equipped_skill {
                if s.name.contains("杀戮") || s.description.contains("80%") {
                    if b_dps > 0.0 {
                        phase_time = ((boss_hp * 0.20) / b_dps) + ((boss_hp * 0.80) / avg_dps);
                    } else {
                        phase_time = (boss_hp / avg_dps) + 60.0;
                    }
                }
            }
        }

        // Boss defeat check: boss direct incoming DPS
        let boss_dmg_per_hit = f64::max(0.05 * boss_atk, boss_atk - self.primary_operator.final_def());
        let boss_incoming_dps = (boss_dmg_per_hit / e_interval) * (1.0 - self.primary_operator.damage_resistance());
        let op_regen = self.primary_operator.hp_regen_per_second();
        let net_incoming_dps = (boss_incoming_dps - op_regen).max(0.0);
        let op_pool = self.primary_operator.final_hp() + self.primary_operator.initial_barrier() + self.primary_operator.skill_barrier();
        let immortality = self.primary_operator.immortality_duration();

        let mut defeat_penalty_time = 0.0;

        // Laios S1 cowers for 15 seconds when facing a Leader/Boss
        if self.primary_operator.name == "Laios" || self.primary_operator.is_char("char_4142_laios") || self.primary_operator.name.contains("莱欧斯") {
            if let Some(s) = &self.primary_operator.equipped_skill {
                if s.name.contains("胆小") || s.description.contains("无法攻击") {
                    defeat_penalty_time += 15.0;
                }
            }
        }

        // Eunectes S3 self-stuns for 5 seconds when skill ends (10s over 2 phases)
        if self.primary_operator.name == "Eunectes" || self.primary_operator.is_char("char_416_zumama") || self.primary_operator.name.contains("森蚺") {
            if let Some(s) = &self.primary_operator.equipped_skill {
                if s.name.contains("钢铁") || s.description.contains("眩晕") {
                    defeat_penalty_time += 10.0;
                }
            }
        }
        if net_incoming_dps > 0.0 {
            let survival_time = (op_pool / net_incoming_dps) + immortality;
            if survival_time < phase_time {
                let deaths_per_phase = (phase_time / survival_time.max(1.0)).floor();
                let redeploy_time = self.primary_operator.final_redeployment_time().max(30.0);
                defeat_penalty_time = deaths_per_phase * redeploy_time * 2.0; // 2 phases
            }
        }

        // 2 phases + 15s revive + defeat penalty
        let total_time = (phase_time * 2.0) + 15.0 + defeat_penalty_time;
        total_time.min(1800.0)
    }
}

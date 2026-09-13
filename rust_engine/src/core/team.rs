// Team Tier List (v1.1) — aggregates the SAME per-operator stats the individual tier list
// already computes into 6 team-level axes. No new combat simulation math lives here; this is
// purely a scoring/aggregation layer on top of the existing engine, per the design in the
// approved plan.
//
// `TeamMemberProfile` is built in `main.rs` (via `build_member_profile`, which calls the
// binary-local `evaluate_single_operator`) since `core` is shared by several standalone
// debug/test binaries that don't have that function in their crate root — this module only
// contains the pure aggregation math, which has no such dependency.

use serde_json::{json, Value};

#[derive(Clone, Debug)]
pub struct TeamMemberProfile {
    pub operator_name: String,
    pub profession: String,
    pub sub_profession_id: String,
    pub rarity: i64,
    // "boss" category — Boss Killing axis
    pub boss_ttc: f64,
    pub boss_leak_pct: f64,
    pub boss_dmg_score: f64,
    pub armor_pen: f64,
    // "normal" category — Lane Holding axis
    pub wave_ttc: f64,
    pub wave_leaks: f64,
    pub block: f64,
    // "general" category — Resistance / Utility / Consistency axes
    pub surv: f64,
    pub heal: f64,
    pub status_resist: f64,
    pub dp_per_sec: f64,
    pub dp_cost: f64,
    pub efficiency: f64,
    pub buffs: f64,
    pub debuffs: f64,
    pub utility_score: f64,
}

/// A rough "did this operator actually do anything" proxy, used to stop near-zero-impact filler
/// (cheap 1★ gadget/trap operators with a token skill and a huge cooldown, or none at all) from
/// gaming axes that would otherwise treat them as free wins — e.g. `efficiency` defaults to 100
/// when no skill is equipped (no skill = no cooldown to speak of), and a 3-DP-cost operator drags
/// down an average deploy cost, regardless of whether they contribute anything in combat. Floored
/// at 1.0 so no member ever has literally zero weight (avoids empty-weighted-average edge cases).
pub fn impact_weight(m: &TeamMemberProfile) -> f64 {
    let combat = m.boss_dmg_score / 200.0;
    let support = (m.buffs + m.debuffs + m.utility_score) / 2.0;
    let healing = m.heal / 300.0;
    let holding = m.block * 20.0;
    let dp_gen = if m.profession == "PIONEER" { m.dp_per_sec * 40.0 } else { 0.0 };
    (combat + support + healing + holding + dp_gen).max(1.0)
}

/// The team's lowest-`impact_weight` member — the one contributing least across every axis
/// combined (damage, support, healing, block, DP generation all feed into that one number), i.e.
/// the operator a "who should I swap out" recommendation should point at first.
pub fn weakest_member(members: &[TeamMemberProfile]) -> Option<String> {
    members.iter()
        .min_by(|a, b| impact_weight(a).partial_cmp(&impact_weight(b)).unwrap_or(std::cmp::Ordering::Equal))
        .map(|m| m.operator_name.clone())
}

fn weighted_avg(pairs: &[(f64, f64)]) -> f64 {
    let total_weight: f64 = pairs.iter().map(|(_, w)| w).sum();
    if total_weight <= 0.0 { return 0.0; }
    pairs.iter().map(|(v, w)| v * w).sum::<f64>() / total_weight
}

fn avg(vals: &[f64]) -> f64 {
    if vals.is_empty() { 0.0 } else { vals.iter().sum::<f64>() / vals.len() as f64 }
}

/// Saturating 0-100 curve: raw material grows the score, but it approaches (never reaches) 100.
/// `k` is the "half-scale" constant — the raw value at which the score crosses 50 — tuned per
/// axis against realistic team compositions (see the Verification step in the plan).
fn normalize(raw: f64, k: f64) -> f64 {
    if raw <= 0.0 { return 0.0; }
    (100.0 * raw / (raw + k)).clamp(0.0, 100.0)
}

fn profession_label(profession: &str) -> &'static str {
    match profession {
        "PIONEER" => "Vanguards",
        "WARRIOR" => "Guards",
        "TANK" => "Defenders",
        "SNIPER" => "Snipers",
        "CASTER" => "Casters",
        "MEDIC" => "Medics",
        "SUPPORT" | "SUPPORTER" => "Supporters",
        "SPECIAL" | "SPECIALIST" => "Specialists",
        _ => "operators of one class",
    }
}

/// A flat 5-tier A-E scale bucketed the entire 80-100 range as a single "A", so two very
/// different teams (say 82 and 96) rendered identically — same complaint the operator/enemy
/// tier lists already solved with an OP/S tier above A. Extending the same idea here: OP is
/// reserved for the genuine top end (97+), S covers the "clearly excellent but not maxed out"
/// range a 96 actually belongs in, matching what a good-but-not-perfect team should read as.
fn grade(score: f64) -> &'static str {
    if score >= 97.0 { "OP" }
    else if score >= 88.0 { "S" }
    else if score >= 72.0 { "A" }
    else if score >= 55.0 { "B" }
    else if score >= 38.0 { "C" }
    else if score >= 20.0 { "D" }
    else { "E" }
}

fn top_contributors<'a>(members: &'a [TeamMemberProfile], key: impl Fn(&TeamMemberProfile) -> f64, n: usize) -> Vec<&'a str> {
    let mut ranked: Vec<&TeamMemberProfile> = members.iter().collect();
    ranked.sort_by(|a, b| key(b).partial_cmp(&key(a)).unwrap_or(std::cmp::Ordering::Equal));
    ranked.into_iter().take(n).filter(|m| key(m) > 0.0001).map(|m| m.operator_name.as_str()).collect()
}

/// Rule-based, human-readable suggestions for closing the team's weakest gaps. Thresholds are on
/// the same 0-100 axis scale as the grades (roughly: <40 is a real gap, <25 is a glaring one).
#[allow(clippy::too_many_arguments)]
fn build_recommendations(
    boss_killing: f64, lane_holding: f64, resistance: f64, utility: f64,
    consistency: f64, reliability: f64, total_heal: f64, total_block: f64,
    vanguard_dp_per_sec: f64, member_count: f64,
) -> Vec<String> {
    let mut recs = Vec::new();

    if resistance < 45.0 && total_heal < 300.0 * member_count / 12.0 {
        recs.push("Low on Resistance with almost no team healing — add a dedicated Medic/Supporter to sustain the front line.".to_string());
    } else if resistance < 45.0 {
        recs.push("Resistance is low despite having some healing — the team's raw EHP is thin; consider a tankier Defender or Guard.".to_string());
    }

    if total_block < 1.5 * member_count / 12.0 * 6.0 {
        recs.push("Very little block coverage — add more Defenders/Vanguards that can hold a lane, or your Lane Holding score will keep suffering.".to_string());
    }

    if lane_holding < 40.0 {
        recs.push("Weak against sustained waves — add an AoE damage dealer (Caster/Guard/Specialist with a wide hit) to clear lanes faster.".to_string());
    }

    if boss_killing < 40.0 {
        recs.push("Weak against single, tough targets — add a dedicated burst/single-target damage dealer, ideally with DEF/RES penetration.".to_string());
    }

    if utility < 45.0 && vanguard_dp_per_sec < 0.3 {
        recs.push("Slow to field — none of your Vanguards generate meaningful extra DP; swap one in for a DP-generating archetype (e.g. Standard Bearer/Agent).".to_string());
    } else if utility < 45.0 {
        recs.push("Low team utility — little in the way of buffs, enemy debuffs, or Crowd Control; add a Supporter/Caster that brings some.".to_string());
    }

    if consistency < 45.0 {
        recs.push("Low uptime — several operators spend more time on cooldown/redeploy than actually fighting; favor kits with a better duration-to-cost ratio.".to_string());
    }

    if reliability < 40.0 {
        recs.push("This team over-specializes: it's strong on a couple of axes but weak everywhere else. Diversify roles rather than stacking more of what it's already good at.".to_string());
    }

    if recs.is_empty() {
        recs.push("Well-rounded team — no major gaps detected across Boss Killing, Lane Holding, Resistance, Utility, or Consistency.".to_string());
    }

    recs
}

/// Aggregates up to 12 member profiles into the 6 team axes requested by the user: Boss Killing,
/// Lane Holding, Resistance, Deploy Time, Consistency, and Reliability (an anti-imbalance
/// corrective so a team can't top the list by excelling at exactly one thing).
pub fn score_team(members: &[TeamMemberProfile]) -> Value {
    // --- Boss Killing: total boss-target damage output, weighted by how fast the top damage
    // dealers actually clear the boss, boosted by their armor penetration. ---
    let mut by_boss_dmg: Vec<&TeamMemberProfile> = members.iter().collect();
    by_boss_dmg.sort_by(|a, b| b.boss_dmg_score.partial_cmp(&a.boss_dmg_score).unwrap_or(std::cmp::Ordering::Equal));
    let top3_boss = &by_boss_dmg[..by_boss_dmg.len().min(3)];
    let total_boss_dmg: f64 = members.iter().map(|m| m.boss_dmg_score).sum();
    let avg_top3_boss_ttc = avg(&top3_boss.iter().map(|m| m.boss_ttc.max(1.0)).collect::<Vec<_>>());
    let avg_armor_pen = avg(&top3_boss.iter().map(|m| m.armor_pen).collect::<Vec<_>>());
    let boss_raw = (total_boss_dmg / avg_top3_boss_ttc.max(1.0)) * (1.0 + avg_armor_pen);
    let boss_killing = normalize(boss_raw, 400.0);

    // --- Lane Holding: how much frontline is held (block, additive — multiple blockers really
    // do combine to hold more lanes) combined with how fast the team's actual wave-clearers
    // clear a sustained wave, penalized by their leaks. `wave_ttc`/`wave_leaks` are each
    // computed as if that operator alone held the lane, so both need to be read off the SAME
    // small set of real front-liners — summing `wave_leaks` (or averaging `wave_ttc`) across all
    // 12 members drags in dedicated healers/pure supports who were never holding that lane in
    // the first place and were never going to clear anything, swamping the real clearers. ---
    let total_block: f64 = members.iter().map(|m| m.block).sum();
    let mut by_wave_speed: Vec<&TeamMemberProfile> = members.iter().collect();
    by_wave_speed.sort_by(|a, b| a.wave_ttc.partial_cmp(&b.wave_ttc).unwrap_or(std::cmp::Ordering::Equal));
    let top_clearers = &by_wave_speed[..by_wave_speed.len().min(4)];
    let avg_wave_ttc = if top_clearers.is_empty() { 300.0 } else { avg(&top_clearers.iter().map(|m| m.wave_ttc.max(1.0)).collect::<Vec<_>>()) };
    // `wave_leaks` is on a 0-100 scale (percent of the 100-enemy onslaught that got through),
    // NOT a 0-1 fraction — dividing by `1.0 + avg_leaks` directly turned a routine 50%-leak
    // clearer into a /51 crush instead of the intended mild ~1.5x penalty, which is what made
    // adding any real lane holder (high block, but a slower, leakier solo clear) look like it
    // tanked the whole axis.
    let avg_leaks_pct = if top_clearers.is_empty() { 0.0 } else { avg(&top_clearers.iter().map(|m| m.wave_leaks).collect::<Vec<_>>()) };
    let lane_raw = ((total_block + 1.0) * 300.0 / avg_wave_ttc.max(1.0)) / (1.0 + avg_leaks_pct / 100.0);
    let lane_holding = normalize(lane_raw, 8.0);

    // --- Resistance: survivability (EHP), team healing SCALED AGAINST THE TEAM'S TOTAL EHP POOL
    // (not an absolute number), and CC/status resistance (the closest proxy to "dodge" this
    // engine tracks — no operator-side evasion stat exists). An absolute `total_heal` number let
    // a single real healer (e.g. one Haruka) push Resistance to 97-99 on a 12-operator team
    // regardless of how much total HP that team actually has to sustain — the same heal number
    // means a lot for a squishy 6-operator core and very little for a team with a huge combined
    // EHP pool, so healing is judged as a FRACTION of the pool it's actually covering. ---
    let avg_surv = avg(&members.iter().map(|m| m.surv).collect::<Vec<_>>());
    let total_heal: f64 = members.iter().map(|m| m.heal).sum();
    let total_surv_pool: f64 = members.iter().map(|m| m.surv).sum();
    let heal_coverage_ratio = total_heal / total_surv_pool.max(1.0);
    let avg_status_resist = avg(&members.iter().map(|m| m.status_resist).collect::<Vec<_>>());
    let resist_raw = avg_surv + heal_coverage_ratio * 4000.0 + avg_status_resist * 300.0;
    let resistance = normalize(resist_raw, 6000.0);

    // --- Utility: how fast the team can afford to keep throwing REAL contributors onto the
    // field (deploy-speed component), PLUS its buffs/debuffs/Crowd-Control utility. Originally
    // this was a separate "Deploy Time" axis using a flat average dp_cost, which a team could
    // game for free by padding slots with cheap, near-zero-impact 1★ gadget/trap operators (3 DP
    // cost, does essentially nothing) — the average dropped, the score went up, despite the
    // operator contributing nothing. Weighting dp_cost by `impact_weight` means only OPERATORS
    // THAT ACTUALLY DO SOMETHING pull the average toward their real cost; free-riding filler
    // barely moves it. Vanguard DP generation still speeds this up, per the user's request that
    // Vanguards matter MORE here. ---
    let total_dp_cost: f64 = members.iter().map(|m| m.dp_cost).sum();
    let avg_dp_cost = weighted_avg(&members.iter().map(|m| (m.dp_cost, impact_weight(m))).collect::<Vec<_>>());
    let vanguard_dp_per_sec: f64 = members.iter()
        .filter(|m| m.profession == "PIONEER")
        .map(|m| m.dp_per_sec)
        .sum();
    let avg_redeploy_time = avg_dp_cost / (1.0 + vanguard_dp_per_sec);
    // The whole-roster figure the UI shows below the radar: literally how long it'd take to
    // field all 12 from a standing start, at 1 DP/sec natural regen plus Vanguard generation.
    let team_full_deploy_time = total_dp_cost / (1.0 + vanguard_dp_per_sec);
    // Half-scale ~18: a team with NO vanguard DP generation (avg_redeploy_time == its own real
    // avg cost, typically ~15-25) lands around a C: only real DP generation (or a cheap roster of
    // operators that actually contribute) pulls it up.
    let deploy_component = (100.0 * 18.0 / (18.0 + avg_redeploy_time.max(0.1))).clamp(0.0, 100.0);

    let avg_buffs = avg(&members.iter().map(|m| m.buffs).collect::<Vec<_>>());
    let avg_debuffs = avg(&members.iter().map(|m| m.debuffs).collect::<Vec<_>>());
    let avg_utility_score = avg(&members.iter().map(|m| m.utility_score).collect::<Vec<_>>());
    let support_raw = avg_buffs + avg_debuffs + avg_utility_score;
    let support_component = normalize(support_raw, 40.0);

    // Deploy speed is deliberately a MINOR slice of Utility now: weighted-averaging dp_cost can
    // only ever be flattened FURTHER DOWN by adding more cheap operators (there's no way for a
    // cheap addition to make a weighted average go up), so a genetic search chasing a heavily-
    // weighted deploy component had a standing incentive to pad slots with extra cheap Vanguards
    // regardless of whether the team needed them — exactly what stacking 5-6 Vanguards was
    // exploiting. Buffs/Debuffs/CC utility dominates instead; deploy speed is still a real,
    // visible signal (and the literal `team_full_deploy_time` figure below the radar) but no
    // longer worth stacking redundant Vanguards over.
    let utility = deploy_component * 0.2 + support_component * 0.8;

    // --- Consistency: skill-active uptime (redeploy-cooldown-relative for Executors), weighted
    // by `impact_weight` for the SAME reason as above — `efficiency` defaults to a perfect 100
    // when NO skill is equipped (no skill = no cooldown to speak of), which is technically
    // correct but trivially "perfect" for an operator who was never going to matter anyway.
    // Unweighted, a couple of zero-impact 1★ traps could single-handedly drag a real team's
    // Consistency average up to 100. ---
    let consistency = weighted_avg(&members.iter().map(|m| (m.efficiency, impact_weight(m))).collect::<Vec<_>>()).clamp(0.0, 100.0);

    // --- Reliability: penalizes a team that's excellent on one axis and weak everywhere else. ---
    let axes = [boss_killing, lane_holding, resistance, utility, consistency];
    let mean = avg(&axes);
    let variance = avg(&axes.iter().map(|a| (a - mean).powi(2)).collect::<Vec<_>>());
    let stdev = variance.sqrt();
    let reliability = if mean > 0.001 { (100.0 * (1.0 - stdev / mean)).clamp(0.0, 100.0) } else { 0.0 };

    // --- Role balance penalty: no per-axis formula fully captures "5 Vanguards is a bad team" —
    // Boss Killing/Lane Holding already gate to their top 3-4 real contributors, so redundant
    // copies of the same class just ride along contributing to nothing while costing nobody
    // anything. A real squad has room for maybe 1-2 Vanguards, 2-3 of a damage class, etc.; more
    // than a handful of the SAME profession crowds out roles the team actually needs. Applied as
    // a direct multiplier on the overall score (not folded into an axis) so it can't be diluted
    // away by everything else looking fine. ---
    let mut profession_counts: HashMap<String, i32> = HashMap::new();
    for m in members { *profession_counts.entry(m.profession.clone()).or_insert(0) += 1; }
    let max_profession_count = profession_counts.values().copied().max().unwrap_or(0);
    let role_balance = (100.0 - ((max_profession_count - 3).max(0) as f64) * 25.0).clamp(0.0, 100.0);

    // A team with zero Vanguards has no fast, cheap way to open a stage — every operator has to
    // wait on the same slow natural DP regen, so the WHOLE squad deploys late regardless of how
    // strong it looks on paper. `deploy_component` (inside Utility, 20% weight of ONE of 6 axes)
    // was much too diluted to actually stop the genetic search from picking an all-non-Vanguard
    // "optimal" team — nothing else in the formula cares whether a real starting field presence
    // exists at all. Applied the same way as the too-many-of-one-class penalty: a direct
    // multiplier on overall, so it can't be washed out by every other axis looking great. Only
    // applies once the team is big enough that "no Vanguard yet" is a real choice, not just an
    // in-progress partial build.
    let has_vanguard = profession_counts.get("PIONEER").copied().unwrap_or(0) > 0;
    let no_vanguard_multiplier = if !has_vanguard && members.len() >= 6 { 0.8 } else { 1.0 };

    let role_balance_multiplier = (0.6 + 0.4 * (role_balance / 100.0)) * no_vanguard_multiplier;

    let overall = avg(&[boss_killing, lane_holding, resistance, utility, consistency, reliability]) * role_balance_multiplier;

    let member_count = members.len().max(1) as f64;
    let mut recommendations = build_recommendations(
        boss_killing, lane_holding, resistance, utility, consistency, reliability,
        total_heal, total_block, vanguard_dp_per_sec, member_count,
    );
    if max_profession_count > 3 {
        let (worst_prof, _) = profession_counts.iter().max_by_key(|(_, c)| **c).unwrap();
        recommendations.insert(0, format!(
            "{} of your {} operators are {} — that's far more than a real team fields at once. Swap the extras for other roles (a real squad rarely runs more than 2-3 of the same class).",
            max_profession_count, members.len(), profession_label(worst_prof)
        ));
    }
    if !has_vanguard && members.len() >= 6 {
        recommendations.insert(0, "No Vanguard on this team — without one, every operator waits on slow natural DP regen and the whole squad opens the stage late. Add at least one Vanguard for a fast, cheap opener.".to_string());
    }

    json!({
        "axes": {
            "boss_killing": boss_killing,
            "lane_holding": lane_holding,
            "resistance": resistance,
            "utility": utility,
            "consistency": consistency,
            "reliability": reliability,
        },
        "grades": {
            "boss_killing": grade(boss_killing),
            "lane_holding": grade(lane_holding),
            "resistance": grade(resistance),
            "utility": grade(utility),
            "consistency": grade(consistency),
            "reliability": grade(reliability),
        },
        "contributors": {
            "boss_killing": top_contributors(members, |m| m.boss_dmg_score, 3),
            // Wave-clear SPEED first (this is what actually gates into the axis score's "top 4
            // clearers" average), block count as a minor tiebreaker only — weighting block 10x
            // let a merely-decent blocker with a so-so wave_ttc outrank a genuinely elite
            // wave-clearer (e.g. a top-2-in-the-whole-roster clearer) in this list, which looked
            // like the clearer "wasn't contributing" even though the axis score itself already
            // correctly credited them.
            "lane_holding": top_contributors(members, |m| (300.0 / m.wave_ttc.max(1.0)) * 10.0 + m.block, 3),
            "resistance": top_contributors(members, |m| m.surv + m.heal * 2.0, 3),
            "utility": top_contributors(members, |m| m.buffs + m.debuffs + m.utility_score + (if m.profession == "PIONEER" { m.dp_per_sec * 50.0 } else { 0.0 }), 3),
            "consistency": top_contributors(members, |m| m.efficiency * impact_weight(m), 3),
        },
        "overall_score": overall,
        "overall_grade": grade(overall),
        "member_count": members.len(),
        "recommendations": recommendations,
        "raw": {
            "team_full_deploy_time": team_full_deploy_time,
            "avg_redeploy_time": avg_redeploy_time,
            "total_boss_dmg": total_boss_dmg,
            "avg_top3_boss_ttc": avg_top3_boss_ttc,
            "total_block": total_block,
            "avg_wave_ttc": avg_wave_ttc,
            "avg_leaks": avg_leaks_pct,
            "avg_surv": avg_surv,
            "total_heal": total_heal,
            "total_dp_cost": total_dp_cost,
            "vanguard_dp_per_sec": vanguard_dp_per_sec,
        }
    })
}

// ---------------------------------------------------------------------------------------------
// Phase B: automatically-generated Team Tier List.
//
// Full combinatorics (C(437,12)) is astronomically infeasible, so this runs a small genetic
// search instead: build a role-diverse candidate pool (top performers per axis, not just top
// overall score — so a dedicated healer or DP-Vanguard makes the pool even if they rank low on
// the individual tier list), then evolve a population of 12-operator teams toward high
// `overall_score`, tracking every distinct team seen along the way as a "hall of fame". This is
// the exact mechanism that can show an operator who looks weak in isolation shining in the right
// team context, which is the whole point of the feature.
// ---------------------------------------------------------------------------------------------

use rand::seq::SliceRandom;
use rand::Rng;
use std::collections::HashMap;

const TEAM_SIZE: usize = 12;
// The candidate pool is now tiny (~40 operators, top 5 per class) compared to when these were
// tuned against a ~214-operator pool — fitness evaluation cost doesn't depend on pool size, so a
// bigger population/more generations buys real extra coverage of that small pool for free
// (still ~3s total) instead of converging on the same handful of teams over and over.
const POP_SIZE: usize = 300;
const GENERATIONS: usize = 150;

/// The search draws from every profile it's given, full stop — no additional cut here. The real
/// gate lives one layer up, in `main.rs`'s `compute_team_tierlist_blocking`: only each class's top
/// 5 individually-ranked operators (~40 total, using the SAME per-operator `score` the main Tier
/// List page ranks by) are ever passed in. A pool this size can't be searched exhaustively
/// (C(40,12) is still ~5.6 billion combinations) but is small enough for the genetic search to
/// explore it far more densely than a larger pool, at the cost of excluding anyone outside their
/// class's top 5 entirely — a deliberate trade the user asked for over the earlier, more
/// inclusive but much less densely-searched ~214-operator B+-tier pool.
fn build_candidate_pool(profiles: &HashMap<String, TeamMemberProfile>) -> Vec<String> {
    profiles.keys().cloned().collect()
}

fn random_team(pool: &[String], rng: &mut impl Rng) -> Vec<String> {
    pool.choose_multiple(rng, TEAM_SIZE.min(pool.len())).cloned().collect()
}

fn tournament_select<'a>(scored: &'a [(f64, Vec<String>)], rng: &mut impl Rng) -> &'a Vec<String> {
    let mut best: Option<&(f64, Vec<String>)> = None;
    for _ in 0..4 {
        let cand = &scored[rng.gen_range(0..scored.len())];
        if best.map(|b| cand.0 > b.0).unwrap_or(true) { best = Some(cand); }
    }
    &best.unwrap().1
}

fn crossover(p1: &[String], p2: &[String], rng: &mut impl Rng) -> Vec<String> {
    let mut union: Vec<String> = p1.iter().chain(p2.iter()).cloned().collect();
    union.sort();
    union.dedup();
    union.shuffle(rng);
    union.truncate(TEAM_SIZE);
    union
}

fn mutate(team: &mut [String], pool: &[String], rng: &mut impl Rng) {
    if !pool.is_empty() && rng.gen_bool(0.5) {
        let idx = rng.gen_range(0..team.len());
        if let Some(candidate) = pool.choose(rng) {
            if !team.contains(candidate) {
                team[idx] = candidate.clone();
            }
        }
    }
}

fn team_tier_from_pct(pct: f64) -> &'static str {
    if pct <= 0.025 { "OP" }
    else if pct <= 0.11 { "S" }
    else if pct <= 0.27 { "A" }
    else if pct <= 0.49 { "B" }
    else if pct <= 0.71 { "C" }
    else if pct <= 0.86 { "D" }
    else if pct <= 0.95 { "E" }
    else { "F" }
}

/// Runs the genetic search and returns up to 150 distinct teams, ranked by `overall_score`
/// descending, each carrying its full `score_team` breakdown plus `members`, `rank`, and a
/// percentile-based `team_tier` (same OP/S/A/B/C/D/E/F scale the individual tier list uses).
pub fn generate_team_tierlist(profiles: &HashMap<String, TeamMemberProfile>) -> Vec<Value> {
    if profiles.len() < TEAM_SIZE { return Vec::new(); }
    let pool = build_candidate_pool(profiles);
    if pool.len() < TEAM_SIZE { return Vec::new(); }

    let mut rng = rand::thread_rng();
    // Light elitism + a steady stream of fresh random "immigrants" each generation — without
    // this the population converges to near-clones of a single optimum within ~20 generations
    // (every "top" team ends up being the same dozen operators reshuffled), which defeats the
    // point of a TIER LIST (a spread of genuinely different viable archetypes, not one team
    // repeated 150 times). Elitism keeps the best found; immigrants + a higher mutation rate
    // keep exploring other role compositions.
    let elite_count = (POP_SIZE / 20).max(2);
    let immigrant_count = (POP_SIZE / 10).max(4);
    let mut population: Vec<Vec<String>> = (0..POP_SIZE).map(|_| random_team(&pool, &mut rng)).collect();
    let mut hall_of_fame: HashMap<String, (Vec<String>, f64)> = HashMap::new();

    for _ in 0..GENERATIONS {
        let mut scored: Vec<(f64, Vec<String>)> = population.iter().map(|team| {
            let profs: Vec<TeamMemberProfile> = team.iter().filter_map(|n| profiles.get(n).cloned()).collect();
            let overall = score_team(&profs)["overall_score"].as_f64().unwrap_or(0.0);
            (overall, team.clone())
        }).collect();
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        for (score, team) in &scored {
            let mut key_parts = team.clone();
            key_parts.sort();
            let key = key_parts.join("|");
            hall_of_fame.entry(key)
                .and_modify(|e| if *score > e.1 { *e = (team.clone(), *score); })
                .or_insert_with(|| (team.clone(), *score));
        }

        let mut next_gen: Vec<Vec<String>> = scored.iter().take(elite_count).map(|(_, t)| t.clone()).collect();
        for _ in 0..immigrant_count {
            next_gen.push(random_team(&pool, &mut rng));
        }
        while next_gen.len() < POP_SIZE {
            let p1 = tournament_select(&scored, &mut rng);
            let p2 = tournament_select(&scored, &mut rng);
            let mut child = crossover(p1, p2, &mut rng);
            mutate(&mut child, &pool, &mut rng);
            mutate(&mut child, &pool, &mut rng);
            if child.len() == TEAM_SIZE {
                next_gen.push(child);
            }
        }
        population = next_gen;
    }

    // Top slice by score, capped so the response/DOM stay manageable — with the candidate pool
    // now just each class's top 5 (~40 operators), the search naturally finds far fewer distinct
    // teams than it did against the old ~214-operator pool, so 500 is both a sane cap AND close to
    // an actually-thorough sampling of what's achievable from this small a roster.
    const RESULT_LIMIT: usize = 500;
    let mut hof: Vec<(Vec<String>, f64)> = hall_of_fame.into_values().collect();
    hof.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    hof.truncate(RESULT_LIMIT);
    let n = hof.len().max(1) as f64;

    hof.into_iter().enumerate().map(|(i, (team, _))| {
        let profs: Vec<TeamMemberProfile> = team.iter().filter_map(|n| profiles.get(n).cloned()).collect();
        let mut result = score_team(&profs);
        let pct = (i + 1) as f64 / n;
        if let Some(obj) = result.as_object_mut() {
            obj.insert("members".to_string(), json!(team));
            obj.insert("rank".to_string(), json!(i + 1));
            obj.insert("team_tier".to_string(), json!(team_tier_from_pct(pct)));
        }
        result
    }).collect()
}

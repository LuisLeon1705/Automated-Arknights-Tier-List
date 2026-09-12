# Changelog

All notable features, fixes and balance changes to the **Arknights Analytical Tier List Engine** are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en-GB/1.1.0/) and this project adheres to [Semantic Versioning](https://semver.org/).

---

## [1.0.3] - Specialization Scoring, Buffs/Debuffs/Utility Split & Enemy-Debuff Self-Nerf Fixes (2026-09-11)

### ⚖️ Scoring Architecture

#### Superlinear specialization curve (fixes "Swiss-army-knife" operators like Skadi/Eunectes outranking real specialists)
- **Root cause**: the composite score summed `w_perf * (val / avg)` linearly across every stat category, so being 30% above average in ten unrelated categories scored the same as being 3x average in one — even though Arknights fields 8-12 operators per team, so maximizing a single axis (armor shred, pure Arts DPS, pure survivability) earns a slot more reliably than being merely "decent" everywhere.
- **Fix**: raised the ratio to `ratio.powf(1.5)`. At `val == avg` this is unchanged (1^1.5 = 1, no weight recalibration needed); above average it grows superlinearly, below average it shrinks superlinearly. Verified: Skadi/Eunectes/Istina held flat or dropped in rank; Lappland the Decadenza, Wiš'adel, Lemuen, Hoshiguma the Breacher and Mantra all climbed.

#### Support split into Buffs / Debuffs / Utility
- The old single "Support" score mixed team buffs, enemy debuffs, and field-control/DP utility into one number that didn't compare like cases. Replaced with three tier list tabs: **Buffs** (team ATK/DEF/RES/Sanctuary), **Debuffs** (enemy DEF/RES shred, CC), **Utility** (DP generation, block/field control, niche). `support`/`support_score` are kept internally for the CSV export and legacy weighting but no longer double-counted in the composite score.
- Summon-slot utility penalty (Ling, Mon3tr-likes) no longer subtracts a flat hardcoded constant; it charges the same weighted contribution an *average-utility* operator would have earned, so it self-scales with the roster instead of needing manual retuning as more operators are added.

### 🐛 Fixes

#### Angelina the Mellow Wish routed through Wiš'adel's hardcoded kit ("Wis" ⊂ "Wish")
- **Root cause**: the Wiš'adel special-case dispatch in `state_rates()` matched `op.name.contains("Wis")` as a loose fallback for the accented "Wiš'adel" spelling — which also matches any name containing the substring "Wis", including **"Angelina the Mellow Wish"**. Her entire real kit was silently replaced by Wiš'adel's damage formula, which is what briefly put her at #1 of the whole roster by a wide margin.
- **Fix**: removed the bare `"Wis"` check (the accented `"Wiš"` and exact `"Wisadel"`/char_id checks already cover the real operator safely). Also implemented her actual Talent 1 mechanic ("attacks deal an additional 25-35% ATK as bonus Arts damage"), which `damage_multiplier()` had been silently dropping (treated as a one-shot burst coefficient since the raw value is ≤ 1.0). Verified against arknights.wiki.gg. Her score fell from #1 (1971) to a still-strong #11 (441.6) — consistent with a genuinely powerful, not-yet-released CN kit.

#### Mantra's S3 (Truth Unchanted) never actually activated
- **Root cause**: the S3 branch matched `s_name.contains("真言") || s_name.contains("震荡")`, neither of which is a substring of her real S3 name **"无言为真"** — so equipping S3 always fell through to the generic S1/S2 fallback, silently dropping her signature Paralysis-overflow chain explosion regardless of skill choice.
- **Fix**: match on her real skill name; recalibrated the (previously flat, ATK-independent) damage numbers to scale with her actual final ATK and the verified 145%/185% ATK values from arknights.wiki.gg.

#### Sanctuary/庇护 credited as permanently active when it's gated behind an ally's HP
- **Root cause**: Tsukinogi, Nine-Colored Deer and Eunectes's Sanctuary talents only trigger when the target's HP crosses a threshold ("生命少于40%时…获得庇护" / Eunectes's own "生命值不高于一半时…获得22%庇护"), but both the team-buff crediting path (`main.rs`) and the self-EHP path (`damage_resistance()`) credited the full value unconditionally, as if it were always up.
- **Fix**: new `Operator::sanctuary_hp_gate_factor()` reads the literal threshold from the talent/skill text (including the word "一半" = "half", not just digit percentages) and discounts by how often that condition is realistically true — a low-HP gate (rare) discounts hard, a high-HP gate (Quercus's "HP > 70%", almost always true) barely discounts, and kits with no HP condition at all (Haruka's per-attack-interval bubble) are untouched. Tsukinogi dropped from #44 to #120 (223.8 → 99.1 score) — now consistent with her real reputation as one of the weaker 5★ supports.

#### CC durations (Fear/Frighten/Stun/Slow) credited even when they're only a per-hit chance
- **Root cause**: Lappland the Decadenza's S1 fear ("浮游单元攻击时有{prob}%几率使目标恐惧") is a probabilistic proc, but `calculate_stat()` has no notion of an associated trigger chance and credited the full duration as guaranteed on every hit.
- **Fix**: new `cc_duration_with_prob()` discounts a CC-duration stat by the trigger probability of a buff sharing the same source skill/talent, when one exists. Also extended the existing Stun/Silence enemy-immunity discount to Frighten and Fear (no dedicated immunity stat exists for them, so `stun_immune_ratio` is reused as the closest verified "hard CC" proxy) — this only changes results when ranking against a tougher enemy tier (Elite/Boss), which is also what makes the category split below meaningful.

#### Enemy-targeted ASPD debuffs mis-applied to the operator's OWN attack speed
- **Root cause**: the raw `attack_speed` blackboard key is normally the operator's own ASPD buff, but three kits reuse the identical key for an **enemy**-targeted ASPD debuff instead — Tragodia's "堕梦" talent ("全场…敌人攻击速度-16"), Mayer's otter talent ("被机械水獭阻挡的敌人攻击速度-25"), and Sesa's S2 ("使目标攻击速度-X"). A negative value landing on `calculate_stat("aspd")` slows the operator down instead of the enemy. This is exactly why **Tragodia's RIT-X module (which upgrades the debuff from -16 to -24) scored *lower* than no module at all** — the "upgrade" was slowing him down more than having nothing equipped.
- **Fix**: `normalize_buff()` now routes a negative `attack_speed` value mentioning "敌人" or "目标" to a distinct `target_aspd_debuff` stat instead, which is credited as a (small) Debuff score contribution rather than crippling `final_interval()`. Module buffs carry no description of their own in this dataset, so module-sourced buffs are now checked against the operator's talent text too (a module almost always just upgrades a value the talent already describes). Verified: Tragodia/Mayer/Sesa's modules now score higher than no module, as they should.

#### Physical EHP formula multiplied by the enemy's raw ATK instead of the DEF-mitigated hit
- **Root cause**: `calculate_ehp_phys_against()` correctly computed `dmg_taken` (post-DEF, floored at 5% of ATK) to figure out how many hits the operator's HP pool survives (capped at 20), but then multiplied that hit count by the enemy's **unmitigated** `e_atk` for the final EHP value instead of by `dmg_taken`. For any operator whose DEF nearly cancels the attacker's ATK (Eunectes's *Iron Will* DEF landing almost exactly on the new Contingency Contract target's ATK), this threw away almost all of the mitigation credit and re-inflated the result using the full raw hit strength — reporting ~28,000-35,000 "survivability" against an attacker she'd barely take chip damage from.
- **Fix**: the final multiplication now uses `dmg_taken` (the actual mitigated per-hit damage), matching what "effective HP absorbed" should mean. Squishy operators (where DEF barely reduces `e_atk`) are essentially unaffected; tanky operators against a target their DEF nearly walls out see EHP drop to a realistic range (Eunectes: ~35,500 → ~11,900 against the toughest target).

### ✨ New: Contingency Contract (CC) target category
- Added as a 6th specialized target profile alongside Normal/Elite/Boss/RA/IS — modeled as tougher than the plain Boss profile across the board (1,300 DEF / 60 RES / 110,000 HP / 1,600 ATK, 90%+ hard-CC immunity), reflecting that CC stacks hazard modifiers on top of a boss-plus-elite-wave field and is the single most demanding permanent mode in the game. Carries its own light scoring bonus favoring burst-against-high-value-targets and CC utility, mirroring the existing RA/IS bonuses.
- `generate_tierlist_pdfs.py` now also generates PDFs for the Buffs/Debuffs/Utility metrics and the Contingency Contract category (previously only Support/Score/DPS/Survivability/Healing metrics were exported).

---

## [1.0.2] - Leaking System (Wave Clear / Boss Killer), Mon3tr Talent Fix & Medic Damage (2026-09-11)

### 🐛 Fixes

#### Mon3tr Bug: Talent 2 never received its Skill 2 buff (Overload)
- **Root Cause**: the `talent_scale` blackboard key (talent multiplier, used by ~30 operators: Mon3tr S2, Ulpianus, Swire, Whislash, Crownslayer, etc.) was normalized into `talent_multiplier`, but `target_talent` was never assigned. The matching inside `Talent::get_effective_buffs` requires `target_talent` to equal the talent's name, so the multiplier **was never applied**.
- **Generic fix (no name-based hardcodes)**: `data_loader.rs` now infers `target_talent` by parsing the skill/module description ("第一天赋", "第二天赋", "第三天赋", or a bare "天赋" → first talent that has buffs). Mon3tr S2 (*策略：超负荷*) now correctly scales its 2nd talent (*战术协同*) by ×2.8: ASPD 22 → **61.6**, and the 2.85 → **1.764s** attack interval is derived by the engine itself (no longer hardcoded in `state_rates`).
- **Support score**: `is_ally_buff` now recognizes heal-targeted ally buff wording ("目标及自身", "周围友方", "其他友方", "治疗跳跃"); ATK/HP `blackboard` buffs with value ≤ 2.5 are scored as ratios (the Construct's +20% ATK aura = 60 pts); and talents in `provided_buffs` now flow through the `get_effective_buffs` pipeline, so the S2 ×2.8 scaling (+61.6 team ASPD) counts toward the support score.
- **Removed the previous hardcoded workarounds**: forcing `skill_index=1` for Mon3tr in `/api/simulate_batch`, the unconditional Mon3tr `return true` inside `is_ally_buff`, and the forced `uptime=1.0` for Mountain (already covered generically by the v1.0.1 Duelist mechanics).

### ⚔️ New Mechanics: Leaking System (Combat Fails)

#### Wave Clearer (10 waves × 10 enemies)
- **15s containment window per wave**: every enemy still alive when the window closes leaks to the defense point (`leaked = 10 − 15/TTC × 10`).
- **Penalty**: each leaked enemy (out of 100) penalizes **1%** of the wave score, applied as a proportional inflation of the total TTC.
- **Physical containment**: a living melee operator whose field block covers the remaining mobs leaks nothing; an operator defeated mid-wave cannot hold anything.
- The arbitrary flat `+150s` penalty for 1-block operators was removed, replaced by the leak model.
- `run_wave_sim` now returns `(total_time, leaked_enemies)`; the `wave_leaks` field is exposed in the JSON and shown on cards in **Wave Clearer** mode.

#### Boss Killer (2 phases + 15s revive)
- **30s per-phase boss travel window**; if a phase exceeds the window, the remaining HP fraction leaks (`1 − window/phase_time`).
- **Melee anchors**: living melee operators extend the containment window by **+15s per block point** (cap 3 → 75s). Ulpianus/S3 and Mountain/S2 hold the boss with zero leaks; defeated operators (redeploy ≥ window) leak **100%** (Laios, Surtr without healing).
- **Executors (fast redeploy)**: the boss walks freely while the operator is off-field (Crownslayer ~73% leak).
- `run_boss_sim` now returns `(total_time, leak_ratio)`; `boss_leak_pct` is exposed and shown in **Boss Killer** mode.

#### Tests
- **`cargo test` was completely broken since v1.0.1**: fixed the `evaluate_single_operator` signature in `test_damage_rankings` and recalibrated the hierarchy names/ordering (`Eyjafjalla the Hvít Aska`, `Lappland the Decadenza`, `Pramanix the Prerita`, `Blaze the Igniting Spark`) over the pure healing channel (raw + elemental restore, no mitigation credit).
- New `test_leaking` binary: verifies the Mon3tr talent↔S2 link and the leak behaviour (Myrtle 100% boss leak, Ulpianus containment, Executor downtime).

### 🐛 "Crit" Operators Proccing Constantly & Iana S1 Running Infinitely (v1.0.2b)

#### Trigger chance (`prob`) destroyed by the blackboard mapper
- **Root Cause**: the CN key `prob` has a double meaning (crit/proc trigger chance vs. physical/arts dodge chance). The `data_loader` remapped it **always** to `arts_dodge`, so `damage_multiplier()` never saw any `prob` and used $p = 1.0$: **every crit/proc effect executed constantly** (Mountain 20%→100%, Bagpipe 28%→100%, Schwarz/Stormeye/Exusiai NC, etc.) — exactly the issue reported in the antigravity chat (walkthrough §6, which was never committed).
- **Fix**: `prob` is disambiguated by the source item description: with dodge/block wording ("闪避/抵挡/格挡") → `arts_dodge`; otherwise it stays `prob` and `damage_multiplier()` turns it into the expected value $\;1 + p\,(s-1)\;$.

#### Modules scaling talents instead of upgrading them (double multiplication)
- Module `atk_scale`/`prob` buffs formed a separate multiplicative group from the talent they upgrade (*1.65 × 1.75 = 2.89* per hit on Mountain FGT-Y). Modules now **merge** into the upgraded talent group: the highest scale (1.75) and the highest proc chance (25%) win → the true expected $\;1 + 0.25 \times 0.75 = 1.19\;$.
- `ep_damage_scale` (one-shot elemental fallout bursts, Blaze's *熔点引爆*) and `atk_scale ≤ 1.0` (deploy procs like Nearl's *不畏苦暗*, chain bounce cadences) are **no longer** applied as permanent per-hit multipliers.

#### Iana S1 (*幻影诡雷*) and proc passives (`sp_type: 8`) firing forever
- **Root Cause**: a passive with `atk_scale 4.0` whose real effect is a burst *when switching to the body double after the hologram is attacked* was treated as a permanent ×4 on **every basic attack** (simulated DPS ≈ 2360).
- **Generic fix** in `state_rates()`: `sp_type 8` skills with conditional-trigger wording ("部署后" / "受到攻击" / "切换" / "立即对") are reconverted to expected value by **duty cycle**: the burst fires ~once per enemy attack interval (or once per deployment for "部署后" kits), not on every hit. Iana S1: 4.0 → ~2.2 effective. Also fixes Hoshiguma (荆棘), Projekt Red, Phantom, Misery and Waai Fu (same bug pattern).
- **Mountain S2 (*横扫架势*)**: "*同时攻击阻挡的所有敌人*" is no longer classified as uncapped AoE (`target_limit = 5`); it is now capped by the operator's **block count**. Mountain's physical DPS normalized from ~4,354 to the ~1,500–1,800 range against standard targets, recovering the walkthrough §6 calibration.
- New `test_crit_proc` binary: verifies `prob` disambiguation, crit expected values (Mountain 1.13 / 1.1875 with module), the S2 block cap and the Iana proc damping.

#### Damage-capable medics actually deal damage now (Kal'tsit, the Reed branch, Folinic)
- **Wandering Medics (行医, branch 404) do NOT attack**: they restore HP/elemental damage across their range but never deal damage to enemies. Hvít Aska is guarded at **0 DPS** across all skills (an earlier iteration of this fix wrongly gave the branch an arts channel; corrected after review).
- **Incantation Medics (咒愈师, branch 405 — the Reed archetype branch)**: the entire branch attacks with Arts damage (50% converted into healing). Verified every member deals damage: Reed the Flame Shadow, Titi, Amiya, Hibiscus the Purifier, Vendela. Reed's S3 check was looking for "*生命之火*" while the real skill name is "*生命火种*" (character order) → now detects both plus the generic "同时攻击…两名" wording → 2-target mode applies.
- **Folinic and explicit enemy-striking medic kits**: Folinic (branch 401 physician) fires "复合药剂弹片" that prioritize enemies, healing allies *and* dealing 200% ATK Arts damage (`attack@atk_scale`, which already merges into the skill-state ATK). `state_rates()` now credits a damage channel to any medic whose equipped skill description explicitly strikes enemies ("对敌人造成…" → Arts/Physical depending on the stated damage type; "伤害类型变为…真实" → True). Folinic: S1 = pure heal buff (0 damage, correct), S2 ≈ 75 Arts DPS while keeping her team healing.
- **Kal'tsit (医师 401)**: her `指令：*` skills carry `attack@atk` modifiers (Mon3tr's strikes) that the engine merged into her ATK but without any damage channel. Now, when the skill **commands the summon to attack** ("*Mon3tr可以攻击阻挡的所有敌人*" → physical; "*伤害类型变为真实*" → true damage), the matching channel is credited: S2 ≈ 259 DPS, S3 ≈ 340 true DPS (the 260%→0% ramping of *熔毁* is approximated at its average by the analytical model).
- **Hvít Aska CN skill matching**: her S1/S3 checks were written against **English** names ("Volcanic Echoes", "5 heal") that the CN dataset never matched → her healing boosts silently never applied. Added Chinese matching ("火山回响", "无声润物", "连发", "额外治疗一名"); her S1 healing throughput rose 168 → 310 HPS as intended.
- **Self-sustain in the defeat checks**: *life-leech* kits that heal themselves on every hit ("攻击治疗自身", Mon3tr S3) now subtract their self-healing from incoming DPS in `run_5_minute_sim`, `run_wave_sim` and `run_boss_sim`. Mon3tr S3 no longer "dies" instantly against the boss: from a 1800s timeout / 100% leak to a **~320s TTC with 60-66% leak** (her real SP uptime against a boss she cannot fully block). *Ally-targeted* damage→healing conversion (Incantation medics like Reed) is correctly NOT counted as personal sustain.
- Guards in `test_crit_proc`: pure healers (Nightingale S1–S3, Mon3tr S1/S2, Hvít Aska, Folinic S1) still deal **0** damage.

---

## [1.0.1] - Duelist Weight & Blocking Fix (2026-09-09)

### 🐛 Balance & Mechanics Fixes

#### Weight-Based Blocking Restriction & Module SP Recovery for Duelist Defenders (Eunectes, Aurora, Cement)
- **Canonical Blocking Rule**: implemented the canonical restriction that no operator can block an enemy whose *weight* exceeds its effective blocking capacity (*block count*).
- **Conditional SP Recovery**: the Duelist archetype trait (*"only recovers SP while blocking an enemy"*) now rigorously validates whether the target can be blocked in base state (1 block):
  - Against **Bosses** (weight 5.0–6.0) and **Elites** (weight 3.0–4.0), which vastly exceed a Duelist's unit block, they **cannot block them**, resulting in $\mathbf{0.0\text{ SP/s}}$ charge with no module or with MOD-Y.
  - Removed the error that assumed guaranteed continuous blocking against any boss without checking its weight.
  - Fixed the charge-phase evaluation so it no longer pre-applies the bonus block granted by the active skill (e.g. Eunectes *Iron Will* +2 block) before it has been activated.
- **MOD-X mechanics (DUA-X / HES-X)**:
  - Modeled the **MOD-X** trait upgrade (`sp_recover_ratio = -0.8`), which restores SP while not blocking at **20% of the normal rate** ($\sim 0.20\text{ SP/s} \times (1 + \text{sp\_recovery\_per\_sec})$).
  - Thanks to this, Eunectes with MOD-X slowly charges against heavy bosses without blocking them and gets her S3 up (2 activations in 300s, 132,895 damage dealt, boss killed at 656s), unlike her unmodded state (timeout at 1800s).
- **MOD-Y mechanics (DUA-Y / HES-Y)**:
  - Keeps the trait with no out-of-block recovery (`sp_recover_ratio = -0.999`), but rewards with superior passive stats (permanent +15% ATK and +15% DEF while blocking and mitigating damage) for combat against blockable enemies.
- **Performance Normalization**:
  - Unable to charge their burst skills against heavy bosses/elites (unless wearing MOD-X), Duelists fight in their base state, reflecting their true empirical performance.
  - While in base state at 1 block, they now properly take the containment-deficit penalty for Defender-class operators against swarms.

#### Normalization & Fix of Infinite Passive Skills (S1)
- **Permanent Activation of Passive Buffs**: added the `Skill::is_passive()` method (`sp_type == "8"` or `"Passive"` with 0 cost and 0 duration) to guarantee stat increases (like the +25% ATK and +25% DEF on Eunectes S1 *Tomahawk*) apply continuously and unconditionally in both base and combat states, fixing the bug where they switched off when `is_skill_active` was false.
- **Efficiency Score (100%)**: infinite passive skills now correctly receive a **100%** efficiency score (previously they fell to `0.0` because the type `"8"` was not recognized instead of `"Passive"`, letting RAW unfairly out-rank S1 in the Tier List weighting).
- **Removal of Arbitrary Multipliers**: cleaned up Eunectes' attack rate in `simulation.rs`, removing the arbitrary `atk * 1.15` factor and letting her canonical blackboard increases model physical damage accurately.
- **Synchronization across Simulators and Editor**: integrated `op.change_state()` into the `/api/simulate` and `/api/simulate_batch` endpoints so the interactive simulator evaluates equipped states in real time with their full stats.

---

## [1.0.0] - Initial Version / Stable Core Release (2026-09-09)

> [!WARNING]
> **Version 1.0.0 Notice**:
> This is the official initial release (1.0.0) of the analytical engine. Given the immense number of simulated combinations (over **426 operators**, **3,000+ skill/module configurations**, and **1,698 enemies** evaluated mathematically across discrete 300-second rotations), small glitches, numerical mismatches in uncommon interactions or edge cases are expected.
>
> Reports of discrepancies or anomalous behaviour are greatly appreciated to keep refining the engine in future revisions.

---

### ✨ Core Features

#### 1. Deterministic Rust Simulation Engine (`rust_engine`)
- **300-second (5-minute) Granular Simulation**: models the full rotation cycle, SP charge times (automatic, on-dealt-hit and on-taken-damage), burst windows and downtime instead of theoretical infinite approximations.
- **Exhaustive State Space**: evaluates every operator across all possible skill (`RAW / No Skill`, `S1`, `S2`, `S3`) and module (`NO MOD`, `MOD-X`, `MOD-Y`, `MOD-D`, `MOD-RA`, `MOD-IS`) combinations.
- **High-Performance Multithreaded Parallelism**: **Rayon**-based implementation, processing and ranking 426+ operators and 3,000+ configurations in under **150 milliseconds**.
- **Canonical 5% Damage Floor Rule**: strict application of *Arknights*' hardcoded rule ensuring physical and arts attacks never deal less than 5% of final ATK after DEF and RES calculations.

#### 2. Specialized Combat Scenarios & Target Profiles
- **General**: composite statistical average of the full enemy dataset.
- **Normal Enemies**: low-defence, high-density swarm scenarios.
- **Elites**: armoured enemies with mid-high RES demanding sustained damage or penetration.
- **Bosses**: high-threat enemies (1,000 DEF, 50 RES, 80,000 HP and heavy offensive power).
- **Reclamation Algorithm (RA)**: base defense and mass control against huge hordes.
- **Integrated Strategies (IS)**: roguelike scenario focused on crit-burst scaling and crowd control.
- **DP Generators (DP)**: specialized benchmark measuring reliability and speed of Deployment Point production.

#### 3. Enemy Threat Tier List (1,698+ Enemies)
- Classification of the entire enemy database into *Normal*, *Elite* and *Boss* categories.
- Modeling of **Threat Score**, physical and arts EHP, direct and skill DPS, dodge and special-mechanic detection (revive, invulnerable phases, taunt).

#### 4. Export & Reporting Suite
- **CSV Download**: instant tabular data export from the web interface.
- **116-PDF Analytical Matrix**: automated generation with ReportLab (`Scripts/generate_tierlist_pdfs.py`), packaged in `data/Arknights_Tier_Lists_PDF.zip`.
  - 84 Operator PDFs (6 scenarios × 14 analytical metrics, 426 rows each).
  - 32 Enemy PDFs (4 categories × 8 threat metrics, up to 1,698 rows each).

#### 5. Web Interface & Sandbox Tools
- **Dark Glassmorphism Design**: interactive SVG charts with cumulative distribution (CDF) curves, quick class/rarity filters, and tier badges (`OP`, `S`, `A`, `B`, `C`, `D`).
- **Operator & Buff Editor**: interactive sandbox to tweak stats, inject team buffs and simulate second-by-second damage curves.
- **Enemy Editor**: interface to create, tune and balance test target statistics.

---

### 🐛 Critical Fixes & Adjustments in v1.0.0

- **Instant vs Infinite Skills**: fixed the bug where offensive instant-cast skills with `duration: -1.0` (Necrass S1/S3, Ch'en S2, Eyjafjalla S2) were misinterpreted as continuous infinite-duration skills, disproportionately inflating their damage over the 300 seconds.
- **Trapmaster Archetype (Dorothy, Wang, Ela, Robin, Frost)**: implemented the full dynamic trap deployment and detonation logic driven by SP recharge intervals, applying real area damage (physical and arts) and enemy DEF/RES reductions.
- **Canonical DP Generation**:
  - Fixed read priority on *Flagbearers* (Myrtle, Elysium, Saileach) so the per-tick cost no longer overwrites the cycle's total DP gain (Elysium generates 162 DP and Myrtle 140 DP over 300s).
  - Resolved passive SP aura stacking that artificially inflated SilverAsh the Reignfrost and Siege numbers.
- **Survival & Hits-to-Kill (HTK) Metrics**:
  - Integrated the *Hits-to-Kill (HTK)* model evaluating how many hits an operator can withstand before falling, considering passive regeneration and sanctuary mitigations.
  - Tuned the score to reward *Defender* toughness and penalize unit-block operators (like Eunectes) in swarm scenarios.
- **Retreat & Redeploy Cycle for Executors (*Fast-Redeploy*)**: implemented the field/bench cycle for specialists like Texas Alter and Yato Alter, simulating their on-field burst window and their off-field recharge timer.
- **Defeat Penalty in Combat**: operators whose survivability cannot outlast incoming enemy DPS suffer defeats that reduce their combat activity factor (*combat uptime*), reflecting the need for defensive support.

# Changelog

All notable features, fixes and balance changes to the **Arknights Analytical Tier List Engine** are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en-GB/1.1.0/) and this project adheres to [Semantic Versioning](https://semver.org/).

---

## [1.3.2] - On-Demand GitHub Actions Server (2026-09-13)

### ✨ New: Use the Team Builder from another device without hosting anything 24/7
- Added `.github/workflows/on-demand-server.yml` — a manually-triggered (`workflow_dispatch`) workflow, not a scheduled/always-on one. It builds the release binary, starts it with `READONLY_MODE=1` (the URL is about to be public — see 1.3.1), opens a free `cloudflared` quick tunnel (no account/signup needed) to it, and prints the temporary public URL straight into the run's log. Stays up for a configurable duration (default 60 min, capped at 340 to stay under GitHub's own 360-minute job limit), then the job ends and the server + tunnel both die with it — nothing left running afterward. The Team Builder's dynamic swap recommendation ("team fixer") works exactly as it does locally through that URL, since it's a real live server for the run's duration, not a static snapshot.
- Trigger from the repo's Actions tab, or `gh workflow run on-demand-server.yml -f duration_minutes=90`.

## [1.3.1] - Security Review (2026-09-13)

### 🔒 Fixed: Arbitrary file write via the Operator Editor's photo upload
- **Root cause**: `save_operator`'s multipart handler took the uploaded photo's filename directly from the request's `Content-Disposition` header and wrote it to `../static/images/{filename}` with zero validation. Since nothing stopped that filename from containing `../`, a crafted upload could path-traverse out of `static/images/` entirely and overwrite ANY file the server process can write to — e.g. `../js/main.js` (persistent, site-wide stored XSS: every visitor runs the attacker's JS) or the data files themselves, bypassing the normal save/delete API altogether. Gated behind `READONLY_MODE` for public deployments (1.2.0), but a fully exploitable arbitrary-file-write on any local/unguarded run.
- **Fix**: added `sanitize_upload_filename` — takes only the final path component (defeats `../`, absolute paths, and embedded separators of either flavor) and requires a plain `name.ext` shape with a real image extension (png/jpg/jpeg/webp/gif) and a safe character set for the name. Verified live: an upload crafted as `filename=../js/main.js` no longer touches `static/js/main.js` at all (the `.js` extension alone is rejected), while a legitimate `test_photo.png` upload still saves normally.

### 🔒 New: Basic hardening from a full security pass
- **Security response headers**: `X-Content-Type-Options: nosniff` (stops a browser from re-sniffing a response as something more dangerous than its declared type — relevant since `/static` serves user-uploaded images), `X-Frame-Options: DENY` (blocks this site being iframed elsewhere — clickjacking), `Referrer-Policy: strict-origin-when-cross-origin`. Verified present on every response.
- **Request body size cap**: 20MB (`DefaultBodyLimit`) — the photo upload endpoint had no limit at all before this, so an arbitrarily large "photo" would be read fully into memory before anything rejected it.
- **Reviewed and found acceptable as-is**: `save_enemy`/`delete_enemy` take a strongly-typed `Json<EnemyData>` body and compare `id` only as a value (never used to build a file path), so no injection/traversal surface there. No secrets or credentials found anywhere in the codebase. CORS is entirely unconfigured (the `tower-http` "cors" feature is enabled but no `CorsLayer` is actually applied) — the safe default (no cross-origin JS can read responses), just worth knowing it's not deliberately configured.
- **Known, accepted gaps** (fine for a personal/local tool, worth knowing before exposing this publicly): no authentication on ANY endpoint — `READONLY_MODE` disables writes but every read endpoint, including the full operator/enemy data dumps, is unauthenticated; no rate limiting beyond the global concurrency cap (1.2.0); no CSRF protection (not exploitable today since there's no session/cookie-based auth to hijack, but would matter if auth is ever added later).

## [1.3.0] - History Now Covers Everything, Not Just the Team Tier List (2026-09-13)

### ✨ New: Full-dataset history snapshots
- The disk cache/history introduced for the Team Tier List (1.2.0) only ever remembered that one result. Every distinct dataset now gets a complete, self-contained snapshot under `data/cache/<data_hash>/`: the Team Tier List result, the full operator Tier List cross-tab (all 8 target categories × 17 ranking metrics — the same shape the "export all categories" CSV uses, 22K+ rows), the full enemy Tier List cross-tab (4 threat classes × 8 metrics), and a copy of the raw `Automated_Operators.json`/`Automated_Enemies.json` themselves — so a historical entry stays fully reproducible even after the live data files move on to something else.
- New endpoints: `GET /api/history/{hash}/team_tierlist`, `/operators`, `/enemies`, `/data/operators`, `/data/enemies`. `GET /api/team_tierlist/history` now lists every dataset with `has_team_tierlist`/`has_operator_tierlist`/`has_enemy_tierlist`/`has_data_snapshot` flags so a client knows what's actually available before fetching.
- The operator/enemy cross-tabs are expensive (roughly 8-12 full-roster simulation passes combined) — computed once in the background the first time ANY dataset is touched (piggybacking on the Team Tier List's own compute-or-cache path) and never blocks the response the caller is actually waiting on. Confirmed live: background snapshot completed in the log within ~15s of the triggering request, all 4 flags true, subsequent reads sub-100ms straight from disk regardless of file size (the operator cross-tab alone is ~43MB).

## [1.2.1] - Enemy Tier List regression fix (2026-09-13)

### 🐛 Fixes

#### Enemy Tier List dropped from ~2000 enemies to ~400
- **Root cause**: the NORMAL/ELITE/BOSS stat floor/cap range added earlier this round (to pick which enemies represent a genuine standard-campaign threat for the *operator* Tier List's scoring baseline — see 1.2.0's `is_outlier_for_tier` entry) was wrongly ALSO applied inside `compute_enemy_tierlist`, the function behind the Enemy Tier List page itself. That range was only ever meant to shape what operators get scored against, not to hide entries from the enemy roster you're actually browsing — CC/IS/RA specials and other out-of-range enemies (the majority of the ~2000-entry roster) were being silently dropped from the page entirely.
- **Fix**: removed the filter from `compute_enemy_tierlist`; it now shows the full roster again, unfiltered. `core::enemy::calculate_enemy_stats_for_tier` (the actual scoring-baseline function) is untouched and still applies the range correctly. Confirmed live: Enemy Tier List back to 2151 entries; operator scoring's `enemy_stats` baseline still reports in-range values (e.g. 5,000 HP for Normal, not a 500K outlier).

## [1.2.0] - Mobile/Tablet Responsive Layout, Colorblind-Safe Palette & Operational Hardening (2026-09-13)

### 🐛 Fixed: Statistical Curves section overflowed sideways on mobile
- **Root cause**: same min-content grid-track bug as elsewhere in this round, just missed here — `.chart-grid-panel`'s mobile override used a bare `grid-template-columns: 1fr` instead of `minmax(0, 1fr)`, so a wide Chart.js canvas kept forcing the "collapse to 1 column" override back out to its original width, pushing the whole panel off-screen next to the operator checklist sidebar.
- **Fix**: `minmax(0, 1fr)`, plus the checklist sidebar's own inline `max-width:250px` cleared on mobile, plus both panels forced into a clean top-to-bottom stack below 640px (they technically fit side by side once both shrank to ~150px, but that's too cramped to actually read).

### 🔒 New: `READONLY_MODE` — disable the Operator/Enemy Editor for public deployments
- Editing writes directly to the JSON data files on disk — fine for local/personal use, but not something a public deployment (e.g. a future GitHub Actions-hosted demo) should expose, since anyone could overwrite or delete real data. Set `READONLY_MODE=1` (or `true`) in the environment and `/editor`, `/enemy_editor`, and their save/delete APIs all return a real 403 with a styled "Función no accesible actualmente" page instead of running — gated via an `axum` middleware on just those routes, so the exact same binary works either way depending on how it's deployed. Unset (the default/local case) is completely unaffected. Verified both modes live.

### ✨ New: Collapsible icon nav, and a real multi-column compact grid for both tier lists
- **Header nav**: 7 links wrapped across 2-3 lines under the logo on a phone — a wall of text before you even reach page content. Below the mobile breakpoint it's now a hamburger-toggled dropdown list instead, with an icon per link.
- **Tier List & Enemy Tier List compact mode**: the initial compact-mode pass (below) still packed cards one per row on a phone, because it relied on a fixed-width card wrapping via flex — a `<select>`-style specificity/sizing quirk some browsers hit meant the row never actually broke into multiple columns. Rebuilt on a real CSS grid (`grid-template-columns: repeat(auto-fill, minmax(64px, 1fr))`) instead, which guarantees 3+ cards per row on a phone regardless of that, plus shrunk the tier-color bar itself (80px/60px -> 34px, rotated text) so it stops eating a third of the row's width on top of the cards.

### ✨ New: Works on mobile and tablet, not just desktop
- The whole app previously had zero responsive breakpoints outside `@media print` — any screen narrower than ~1400px just got a squeezed desktop layout, and several real bugs made it actively broken (not just cramped) below ~1024px: a `<select>` with a long option (e.g. "General (Composite Dataset)") forced horizontal scroll on its own regardless of wrapping, `.dashboard-grid`/inline grids used a bare `1fr` track that refuses to shrink below its content's min-content size (the same default-min-width:auto behavior flex items have) so "collapse to 1 column" overrides didn't actually stop overflow, a `.tabs` bar (Operator/Enemy Editor's General/Combat/Skills/...) never wrapped, the Enemy Editor's two-panel layout had no breakpoint at all, and Chart.js canvases kept their JS-assigned pixel width regardless of container size.
- Added real tablet (≤1024px) and phone (≤640px) breakpoints across `static/css/style.css` and each template's own styles, plus generic safety-net rules (`[style*="grid-template-columns"]`, `[style*="display:flex"]`) for the many one-off inline layouts scattered through the editor/comparison pages that aren't practical to convert to shared classes one at a time.
- **Team Builder**: on mobile, your team/radar/score now renders ABOVE the operator picker (previously DOM order put the full ~440-operator search grid first, meaning scrolling past it just to see your own team) — visual order swap via CSS `order`, no HTML restructuring needed.
- **Tier List**: added a COMPACT VIEW / DETAILED VIEW toggle — compact mode shows just each operator's portrait + name (defaults on for phone-width screens on first visit, remembered via `localStorage` after that), instead of every card's full 6-stat breakdown, skill/module tags, and score pill forcing a very long, small-touch-target scroll on a phone.

### ✨ New: Colorblind-safe palette
- The tier color scale (`.tier-OP`..`.tier-F`, used on the operator/enemy tier lists and Team Builder grades) was a straight red-orange-yellow-green-cyan hue rotation — exactly the range that collapses into a near-indistinguishable smear for red-green colorblind users (deuteranopia/protanopia, the most common forms), right across the A/B/C/D range most operators land in. Replaced with a Viridis-derived ramp (the perceptually-uniform palette matplotlib defaults to for this exact reason) that varies in lightness as well as hue, so tier stays readable even if hue can't be distinguished at all. Each tier's text color is now also set explicitly for contrast (a fixed black label was unreadable against the new palette's dark end).
- The Direct Comparisons page's per-operator chart line colors were a red/blue/green/yellow/pink/purple mix that could put a red and green line side by side — replaced with the Okabe-Ito palette, designed specifically to keep every color distinguishable from every other under all common forms of color blindness.

### 🚀 New: Operational hardening (for running this unattended, e.g. a future GitHub Actions workflow)
- **Content-addressed Team Tier List cache**: `/api/team_tierlist` now hashes the actual bytes of the operator/enemy data files (FNV-1a, deterministic across machines — unlike the previous mtime+size fingerprint, which resets on every fresh checkout) and checks in-memory, then an on-disk result (`data/cache/team_tierlist/<hash>.json`), before ever re-running the expensive genetic search. Verified: first request 1.9s (computed), repeat 54ms (memory), after a full restart 61ms (disk, zero recomputation). Every computation is also logged to a browsable history (`GET /api/team_tierlist/history`).
- **Request timeout** (30s, real 408 response) and a **global concurrency limit** (8 at once) so a burst of simultaneous heavy requests queues instead of all hitting the CPU in parallel — the specific concern for a small CI runner.
- **Error pages**: unmatched routes return a real styled 404, and a panic inside any handler is caught and turned into a 500 page instead of silently killing that request with only a stderr line nobody's watching.

## [1.1.1] - Bug Fixes: Kit-Scoring Gaps, Team Builder Recommendations & Special-Mode Exclusion (2026-09-13)

### ✨ New: Disk-backed Team Tier List cache + operational hardening (for running this unattended, e.g. GitHub Actions)
- **Content-addressed caching**: `/api/team_tierlist` now hashes the actual bytes of `Automated_Operators.json`/`Automated_Enemies.json` (FNV-1a, deterministic across machines/Rust versions — unlike the old mtime+size fingerprint, which reset on every fresh checkout) and checks in-memory, then an on-disk result (`data/cache/team_tierlist/<hash>.json`), before ever re-running the expensive genetic search. A result computed once is never recomputed for the same data again, on this machine or any other. Verified live: first request 1.9s (computed), repeat request 54ms (memory), and after a full server restart, 61ms (disk, zero recomputation).
- **History**: every computation is logged to `data/cache/team_tierlist/history.json` (hash, timestamp, team count, newest first, capped at 200) and browsable via the new `GET /api/team_tierlist/history`.
- **Request timeout**: every request now fails cleanly with a 408 after 30s instead of hanging indefinitely — important once nothing's watching the terminal to notice a stuck request.
- **Concurrency limit**: at most 8 requests run at once server-wide (`tower::ServiceBuilder::concurrency_limit`); a burst beyond that queues instead of all running in parallel — the specific concern for a GitHub Actions runner (a couple of CPU cores) being asked to do several full-roster/genetic-search computations simultaneously.
- **Error pages**: unmatched routes now return a real styled 404 instead of axum's default plain-text response, and a panic inside any handler (a bad `.unwrap()`, an out-of-bounds index) is caught and turned into a 500 page instead of silently killing that request's task with only a stderr line nobody's watching.

### 🐛 Fixes

#### Sakiko Togawa's S2 "Piano" stance was never actually simulated — both switch modes scored identically
- **Root cause**: S2 (满月的舞会) is a real in-game switch between two stances — Piano (the initial/default one: +110% ATK, notes pierce through and hit every enemy they pass, Physical) and Organ (+140 ASPD, single-target, Arts). The engine generated two rows tagged `S2-1`/`S2-2` to represent them, but the damage-formula code only ever implemented the Organ branch — the "S2-1" row was just a clone of Organ's numbers relabeled, so Piano's pierce/multi-target damage (exactly what should make her a strong Lane Holder) was never modeled at all.
- **Fix**: added a real Piano-mode damage branch (Physical, `target_limit = 3` to reflect the pierce hitting multiple enemies in the note's path instead of locking to one target) and a `skill_variant` selector on `Operator` so the two stances now run as two genuinely independent simulations. Confirmed via live data: her Piano-tagged row's wave-clear time dropped from ~920s (previously identical to Organ) to ~130s.

#### Push/pull was credited even when the operator's force couldn't actually move the target
- **Root cause**: the niche-score push/pull term computed `force_weight = pp_force - avg_enemy.weight + 2.0` then floored it at `f64::max(0.5, force_weight)` — so even when an operator's push/pull force was well below the target's weight (a real in-game no-op: the enemy just doesn't budge), the formula still guaranteed a minimum non-zero credit. Also stacked a flat `+2.0` margin on top of every comparison, further inflating pulls/pushes that shouldn't work at all.
- **Fix**: credit is now `(pp_force - avg_enemy.weight).max(0.0)` — exactly 0 whenever force doesn't exceed the target's weight, no guaranteed floor. Also cut the overall scale (0.5 -> 0.25), since even a push/pull that DOES work is inherently positioning/map-dependent (needs open space behind the target) rather than reliably usable on every stage the way a straightforward CC duration is. Confirmed live: 12 operators' niche score dropped (e.g. Gladiia 1.0 -> 0.0, Shaw 3.0 -> 0.0), all cases where their force was actually below the average enemy's weight.

#### "Who should you swap next" never actually recommended a Vanguard, even with zero on the team
- **Root cause**: the swap-candidate search always chased whichever of the 5 scored role axes was lowest — but a missing Vanguard isn't fully captured by any of them (it's diluted inside 20% of Utility), so the "No Vanguard on this team" warning and the swap suggestion were completely disconnected: the recommendation correctly flagged the gap, while the suggestion kept optimizing for something unrelated (Consistency, etc.) and never once pointed at an actual Vanguard.
- **Fix**: when the team has zero Vanguards (6+ members, same threshold as the score penalty above), the swap search now prioritizes filling that gap directly — candidates are restricted to Vanguards, ranked by DP generation. Confirmed live: a no-Vanguard team's suggestion changed from unrelated Consistency picks to Tulip/SilverAsh the Reignfrost/Surfer, all Vanguards.

#### The auto-generated Team Tier List could crown a team with zero Vanguards as "the best"
- **Root cause**: the genetic search's fitness function IS `score_team`'s `overall_score`, and nothing in that formula meaningfully cared whether the team had a Vanguard at all — `deploy_component` (the only thing tracking DP generation) is just 20% weight of ONE of 6 averaged axes (Utility), diluted to a negligible fraction of `overall_score`. A team that's strong everywhere else could top the list with no fast/cheap way to open the stage at all, forcing the whole squad to wait on slow natural DP regen.
- **Fix**: added a direct 0.8x penalty multiplier on `overall_score` for any 6+ member team with zero Vanguards (`PIONEER`), the same mechanism already used for "too many of one class" — plus a matching Team Builder recommendation. Since the GA's fitness function IS this same `overall_score`, the fix applies to both the interactive Team Builder and the auto-generated Team Tier List from one change.

#### Perfumer's burst heal was modeled as a repeating heal-over-time, inflating her healing ~2x
- **Root cause**: the generic Abjurer healing formula (`direct_hps = heal_targets * atk * heal_scale / interval`) assumes every Abjurer heal is continuous, re-applied at the operator's own attack cadence — true for most of them, but Perfumer's S1 literally reads "立即进行一次治疗" ("immediately perform ONE heal"): a single 500%-ATK burst per cast, rechargeable twice via SP (`sp_type: INCREASE_WITH_TIME`, nothing to do with her attack interval at all). Dividing that 500% scale by her ~1-2s attack interval as if it re-fired every attack overcounted her healing by roughly the same margin a genuine repeating heal would.
- **Fix**: detects the "immediately perform ONE heal" burst-cast pattern generically (not Perfumer-specific — any Abjurer using the same phrasing benefits) and uses the skill's own `sp_cost` as the recast cadence instead of attack interval. Confirmed live: her S1 heal dropped from 262K to 114K (S -> A tier), a realistic burst-support profile instead of a phantom continuous healer.

#### Ines's signature CC and SilverAsh the Reignfrost's Cold/Frozen mechanic scored as literally nothing
- **Root cause**: this engine only tracks a fixed, hardcoded list of CC stats (stun/frighten/fear/slow/silence duration) — two real, load-bearing debuffs weren't on that list at all: Ines's talent "影织" roots (束缚) EVERY enemy for 5s the first time she damages them (not a single-target lock — across a full wave that's functionally near-blanket AoE CC), and SilverAsh the Reignfrost's S2 applies "寒冷" (Cold), her kit's whole gimmick (2 stacks converts to a full Frozen lockdown in-game). Neither has a dedicated tracked stat, so both contributed exactly 0 to niche/support/debuff scoring — Ines's single strongest kit element and half of Reignfrost's identity were invisible to the scorer.
- **Fix**: added a `cold` duration check (a real structured blackboard stat, so this benefits anyone else using it too — not just SilverAsh) alongside the existing stun/frighten/fear/slow checks, plus a specific credit for Ines's per-enemy root given it has no generic stat key this engine tracks anywhere. Confirmed live: Ines went B -> A tier (93 -> 136 score), SilverAsh the Reignfrost climbed within A tier (132 -> 158).

#### Consistency's uptime metric gave a false 100% to burst skills that actually have real downtime
- **Root cause**: two false-positive patterns in the same `eff_score` heuristic. (1) `is_infinite_or_toggle()` text-matches "持续时间无限" ("unlimited duration") as "sustainable, always credit 100%" — but Surtr's S3 "黄昏" uses that exact phrase to describe an HP-drain EXECUTE mode ("逐渐流失生命", gradually loses HP until it kills her), not a toggle; the "no fixed timer" is because it runs until she dies, not because it's free to keep up. (2) `is_passive()`'s `sp_type == "8"` catch-all assumes that sp_type always means "always active" — but Nearl the Radiant Knight's S2 "逐夜烁光" is a free-deploy-trigger-then-FORCED-auto-retreat-with-EXTENDED-redeploy-time skill, the opposite of cooldown-free. Both scored a flat 100% efficiency for what are actually burst windows with real downtime, which fed straight into Team Builder's Consistency axis and its "who to swap in" recommendations — recommending operators whose actual uptime is nowhere near what the number implied.
- **Fix**: added text-pattern overrides ahead of the infinite/passive check — a "死亡计时" skill (detected via "流失生命") now uses its stated duration vs SP cost instead of an automatic 100, and a "forced auto-retreat" skill (detected via "自动撤退" + a redeploy-time-extension term) uses duration vs `final_redeployment_time()` instead. Confirmed live: Nearl's best config went from a false 100.0 to 27.2, Surtr's from 100.0 to 92.3 (a smaller correction since her downtime is more occasional — this doesn't model her stated escalating 70s cooldown after a second death, just the base death-timer tradeoff). The Consistency swap recommendation for a real team immediately stopped suggesting either of them.
- **Same bug, whole Executor archetype**: every Executor (Texas the Omertosa, etc.) runs entirely on `sp_type == "8"` skills too — an Executor's whole kit is a kill-chain that only keeps re-triggering while she's landing killing blows, so breaking that chain means real idle downtime before it can go again. The duration-vs-redeploy correction was previously wired up ONLY inside Team Builder's per-member override (`build_member_profile`), so the base tier list — and anything ranking off it, including the swap-candidate search above — still saw every Executor at a false 100%, which is exactly how Texas the Omertosa ended up recommended as a Consistency pick right after Nearl and Surtr were fixed. Moved the same correction into `evaluate_single_operator` itself so it applies everywhere uniformly. Confirmed live: Texas's efficiency dropped from 100.0 to 30-40% across her configs, and she no longer appears in the Consistency swap recommendation either.

#### Ray dominated Team Builder's "Top contributors" lists (Boss Killing, Consistency) regardless of role fit
- **Root cause**: Ray's hardcoded damage formulas used FLAT constants (`11000.0 * interval` for S3, `2500.0 * interval` for her base attack) completely untethered from her actual ATK stat — every Ray dealt the exact same fixed damage no matter her gear/trust/potential. That inflated her `boss_dmg_score`, which inflates `impact_weight` (the number Team Builder's "Top contributors" ranks by on EVERY axis, not just damage ones) — so a Hunter with no business topping Consistency could still crowd out the operators actually driving that axis. Also found along the way: her S1/S2 branches checked English skill names (`"Parting Shot"`) that never appear in this dataset (skills are named in Chinese), so both were dead code silently falling back to an arbitrary `atk * 2.2 * 1.15` guess.
- **Fix**: rewrote all 4 states (base, S1 "脱身矢", S2 "广域警觉", S3 "得见光芒") to scale off her real ATK using the actual blackboard `atk_scale` values (450%/no special scale/330%), fixing the dead name checks along the way. Confirmed live: she no longer appears in the Boss Killing or Consistency "Top contributors" list for a team she isn't actually carrying.
- **Follow-up correction**: the first pass credited S3's 330% ATK scale but not its bigger real effect — "装填间隔大幅缩短" (reload interval hugely cut). Ray is a Hunter, an archetype built entirely around a fire-a-few-shots-then-reload cycle; this engine has no generic model for that downtime, so crediting only the per-hit scale at her normal cadence silently dropped S3's actual headline effect (removing the reload pauses and just firing continuously), undershooting an archetype that's specifically about reaching huge burst numbers on this skill. Now applies the blackboard's own `reload_interval: -1.2s` directly as a cadence speedup alongside the 330% scale — she's back to OP-tier Boss Killing, grounded in her real kit instead of a flat constant either way.

#### Team axis grades flattened the entire 80-100 range into a single "A"
- A team scoring 82 and one scoring 96 both just showed "A" — the same complaint the operator/enemy tier lists already solved with an OP/S tier above A. Extended the same idea to team grades: **OP** (97+), **S** (88-97), then A/B/C/D/E below, so a genuinely great-but-not-maxed 96 now reads as S instead of getting lumped in with an 80.

#### Swap recommendation could suggest a candidate that improved the target axis but tanked the overall score
- **Root cause**: `axis_swap_candidates` ranked purely by the single weakest axis's own metric, with zero awareness of `score_team`'s profession-balance penalty (`role_balance_multiplier`, which kicks in above 3 of the same class). A team with 3 Guards already and Gladiia (a Specialist) flagged as the weakest link could get recommended Blaze — a genuinely strong, well-rounded Guard — as the swap-in, which technically raised Consistency but pushed the team to 4 Guards and dropped the OVERALL score by 8 points (81 -> 73) from the class-stacking penalty alone. A locally-correct suggestion that made the team worse.
- **Fix**: `axis_swap_candidates` now takes the team's post-removal profession distribution and excludes any candidate whose class is already at 3 (team.rs's own soft cap) in that remaining roster. Confirmed live on the exact reported team: Blaze no longer appears in the recommendation; a Sniper (Narantuya) is suggested instead.

### ✨ New: "Who to swap next" recommendation
- `POST /api/team_score` (4+ members) now returns a `swap_recommendation` field: the team's lowest-`impact_weight` member (the one contributing least across damage/support/healing/block/DP combined) paired with 3 roster candidates who are specifically strong on the team's single weakest axis (Boss Killing/Lane Holding/Resistance/Utility/Consistency) and aren't already on the team — a direct answer to "who should I swap out, and for who" instead of just showing the weak axis and leaving the fix as an exercise.

#### Team Builder's Lane Holding axis collapsed when a real lane holder (high block, slower solo clear, more leaks) was locked into the team
- **Root cause**: `wave_leaks` is a 0-100 field (percent of the 100-enemy onslaught that got through), but `score_team`'s Lane Holding formula divided by `1.0 + avg_leaks` treating it as a 0-1 fraction — a routine 50%-leak clearer (normal for a tanky blocker who kills slower than a dedicated AoE nuker, e.g. Ulpianus's S2) turned the divisor into `1 + 50 = 51` instead of the intended mild `1 + 0.5 = 1.5`, crushing the whole axis by ~34x whenever a genuine lane holder made the team's top-4-by-speed cut.
- **Fix**: divide by `1.0 + avg_leaks / 100.0` instead, matching the field's actual scale.

#### Wave Clearer's leak check penalized genuine lane holders as hard as a pure burst check
- **Root cause**: `run_wave_sim` compared every one of the 10 waves in a stage against the exact same flat 15s containment window, from wave 1 onward — but real stages don't drop the full onslaught on the lane at a constant, already-fast cadence from the first second; the opening waves trickle in with much more travel time before the pace ramps up toward the end. A flat window for all 10 waves rewards pure burst/kill-speed identically to a sustained blocker, understating operators like Ulpianus's S2 (block 3, steady AoE) relative to a faster single-target nuke like her S3.
- **Fix**: the per-wave containment window and the break between waves now both ramp down on a quadratic (non-linear) curve instead of a flat value — window: 40s -> 33.7 -> 28.2 -> 23.3 -> 19.3 -> 15.9 -> 13.3 -> 11.5 -> 10.4 -> 10s; break between waves: 10s -> 8.1 -> 6.5 -> 5.1 -> 4.0 -> 3.1 -> 2.5 -> 2.1 -> 2s (9 gaps).

#### Tier List target baselines and the Enemy Tier List itself included special-mode enemies mislabeled as standard NORMAL/ELITE/BOSS
- **Root cause**: a handful of entries in `Automated_Enemies.json` carry a plain NORMAL/ELITE/BOSS `tier` label but stats that actually belong to a Contingency Contract/Integrated Strategies/Reclamation Algorithm-only encounter, on either extreme — e.g. `enemy_2093_skzams` is tagged NORMAL with ~500K HP, while other entries are near-zero-stat exceptions that don't represent a real threat of their tier either. Left in, these outliers dragged the population-average enemy stats every operator gets scored against (and the Enemy Tier List's own NORMAL/ELITE/BOSS rankings) far out of line with what those tiers actually look like in standard campaign content.
- **Fix**: added per-tier stat floors AND caps (NORMAL: 1K-15K HP / 100-1000 DEF / 10-30 RES / 100-1000 ATK, ELITE: 5K-30K HP / 500-2500 DEF / 30-60 RES / 500-2000 ATK, BOSS: 15K-200K HP / 1000-4500 DEF / 45-90 RES / 2000-4000 ATK) — any enemy outside its tier's range on any one stat is excluded from both the operator-scoring baseline (`core::enemy::calculate_enemy_stats_for_tier`) and the Enemy Tier List page itself (`compute_enemy_tierlist`). The ATK floor is skipped for enemies reporting `atk = 0`, since many real bosses/elites legitimately deal all their damage through skills/auras rather than a basic attack — that's not the "weak outlier" this floor is meant to catch.

#### Team Builder / Team Tier List recommendations could point at special-mode-only operators
- **Root cause**: `is_excluded_operator` (which drops Reserve Operator placeholders and a specific char_id range from special-mode-exclusive kits, e.g. Reclamation Algorithm-only operators) was only ever applied to the main Tier List's per-raw-entry scan — the swap-candidate search built its own separate pool and never checked it. Worse, some special-mode operators share a display `name` with an unrelated standard-roster entry (e.g. "Tulip" has two raw records: a Reclamation Algorithm-exclusive 6★ correctly inside the excluded ID range, and a separate 5★ that isn't) — so even a naive per-row exclusion check wouldn't have reliably kept every "Tulip" result out, since the non-excluded duplicate could still surface under the same name.
- **Fix**: `axis_swap_candidates` now excludes any operator name that ANY raw entry under that name would have been excluded for, closing both the missing-check gap and the duplicate-name leak in one pass. Confirmed live: Tulip no longer appears in a Vanguard swap suggestion.

---

## [1.1] - Amiya's Alternate Forms & CSV Export Overhaul (2026-09-13)

### 🐛 Fixes

#### Amiya's Guard and Medic alternate forms were invisible everywhere
- **Root cause**: `data/Automated_Operators.json` genuinely has 3 distinct entries for Amiya (Caster/`char_002_amiya`, Guard/`char_1001_amiya2`, Medic/`char_1037_amiya3`) — but all 3 shared the literal `name` field `"Amiya"`. Since every lookup in this app is name-keyed (`get_operator(name)`, the Tier List's per-name dedup, the Team Builder's roster search), any reference to "Amiya" could only ever resolve to whichever entry appears first in the array — the Caster form. The Guard and Medic forms existed in the data and were fully simulated, but had no way to ever be selected, searched for, or shown as their own Tier List row.
- **Fix**: renamed the two alternate-form entries to `Amiya (Guard)` and `Amiya (Medic)` directly in the data file, leaving the original Caster form as plain `Amiya`. All three now appear, searchable and selectable, everywhere in the app. (Turned out to matter more than expected — see the CSV export fix below.)

#### Operator CSV export always showed a blank Tier and a Score of 0.00
- **Root cause**: `export_tierlist_csv` re-simulated every operator itself and sorted by a `"score"` field it read off each row — but `evaluate_single_operator`'s raw output never HAS a `score`/`tier` field; those are only computed later, in the population-relative percentile pass (`compute_tierlist_for_category`, used by `/api/tierlist_data`). The export's own sort key was always `0.0`, so the CSV's "Rank" column was really just insertion order and "Tier" was always blank.
- **Fix**: the export now calls the same `compute_tierlist_for_category` the JSON API and the Tier List page itself use, so the CSV's Rank/Tier/Score exactly match what you see on screen. First ranking exposed by this fix: **Amiya (Guard)**, invisible until the fix above, landed at #1 overall on the General category — which led straight to the next bug.

#### Amiya (Guard)'s S2 (and Amiya (Medic)'s equivalent) simulated as a repeating skill instead of a real once-per-battle execute
- **Root cause**: both skills' raw text literally says "整场战斗中该技能只能释放一次" ("this skill can only be activated once in the entire battle") — but nothing in the STRUCTURED skill data (sp_cost/duration/sp_type all look like an ordinary repeating skill) marks that, so the simulation recharged and re-fired what's meant to be a single execute-type nuke roughly 5 times over the 300s window, inflating her Arts damage ~5.3x (882K real vs. 4.65M simulated) and making her a fabricated #1 overall — exactly what the user flagged: "el personaje en sí no es tan bueno, específicamente porque solo puede usar la S2 una sola vez, igual que su medic variant." A whole-roster search turned up exactly these 2 skills with this exact wording — no other operator kit uses this mechanic.
- **Fix**: `SimulationEnvironment::get_cycle_times_custom` now detects this description text generically and caps the skill to exactly 1 activation for the full simulation, regardless of how much SP would otherwise regenerate. Both Amiya alt-forms now score realistically (still strong, no longer wildly overcounted).

#### Enemy Tier List's "DESCARGAR PDFS (.ZIP)" button was a dead link
- Left behind when the PDF/ZIP export endpoint was removed (see v1.1's Data Export entry) — this button pointed at the same now-deleted route. Replaced with **EXPORT CSV** / **EXPORT ALL CATEGORIES**, matching the operator Tier List (there was no enemy CSV export at all before this).

#### Team Builder's "Top" contributors list for Lane Holding could omit the team's actual best wave-clearer
- **Root cause**: the Lane Holding *score* correctly gates to the team's top 3-4 fastest wave-clearers, but the separate "Top contributors" display list used a different formula (`block × 10 + 1/wave_ttc`) that let a merely-decent blocker with a high block count outrank a genuinely elite clearer (e.g. one ranked #2 of the entire 400+ operator roster on wave-clear speed) in the displayed list — making it look like that operator "wasn't contributing" when the axis score already credited them correctly.
- **Fix**: re-weighted the contributor formula to lead with wave-clear speed (matching what actually drives the score), with block as a minor tiebreaker only.

### ✨ New: All-categories CSV export (operators and enemies) — now with the full cross-analysis
- **EXPORT ALL CATEGORIES** on the Tier List page downloads one CSV covering all 8 target categories (General/Normal/Elite/Boss/RA/IS/CC/DP) with a `Category` column, instead of one category at a time.
- The Enemy Tier List gained CSV export for the first time: **EXPORT CSV** (current threat class) and **EXPORT ALL CATEGORIES** (All/Boss/Elite/Normal in one file — each threat class is re-ranked against just that subset, so they're genuinely different rankings, not just a filtered view of "All").
- **Second axis added**: target category alone is only half of what each Tier List page lets you view live — the other half is which of the page's own ranking-metric dropdowns you're sorted by (17 options for operators: General/DP/Wave/Boss/Phys/Arts/Ele/Healing.../Buffs/Debuffs/Utility/Survivability...; 8 for enemies: Threat Score/HP/EHP/DPS/ATK/DEF/RES/Weight). The "all categories" exports now carry a `Rank (metric)`/`Tier (metric)` column PAIR for every one of those, computed within each row's own category — the full 8×17 cross-tab for operators and 4×8 for enemies in one file each, so a single row shows not just its overall rank but how it stacks up on every other axis too (e.g. filter to `Category=boss` and directly compare a row's `Rank (boss)` against its `Rank (arts_dmg)` to see a specialist-vs-generalist tradeoff). A blank Rank/Tier pair means that row was gated out of that metric entirely (currently only `dp`, which — matching the page itself — only ranks operators who actually generate DP).

---

## [1.1] - Vanguard DP Weighting, Filters & Class Icons, Homepage Restructure & Operator Count Fix (2026-09-12)

### ⚖️ Scoring Architecture

#### Vanguards under-ranked despite being a practically mandatory class
- **Root cause**: two compounding issues. First, a DP-trickle bug affected 15 Vanguard operators whose DP-generation buffs (from talents/skills feeding `dp_recovery`/`dp_recovery_per_sec`-style blackboard keys) weren't being picked up by `expand_dp_trickle_buffs()`, so their signature "class-defining" contribution was silently worth zero. Second, even correctly-credited DP generation was weighted the same as any other secondary stat, which undervalues it — a Vanguard's entire practical role in a team composition is enabling everyone else's DP economy, not doing direct damage/survivability numbers comparable to a Guard or Defender.
- **Fix**: fixed the trickle-buff detection so all 15 affected Vanguards now correctly credit their DP-generation kit. Also raised the DP-generation stat's weight (`w_perf = 36.0`) in the **General** category to match the weight it already carried in the dedicated **DP Generators** category, so a Vanguard's core value proposition is no longer diluted relative to specialists being judged on their own specialty stat.

### 🐛 Fixes

#### Operator counts disagreed across the app (460 raw / 447 loader / 437 tier list)
- **Root cause**: `Automated_Operators.json` contains 460 raw entries, but two families of "Exclusive Operators" (per arknights.wiki.gg) are never actually part of a player's roster — only obtainable pre-built inside Integrated Strategies' Temporary Recruitment or Stationary Security Service. **Reserve Operators** (13 generic, skill-less, talent-less class stand-ins, already excluded) were the only ones filtered out. The second family, **Elite Operators** — max-level/promotion pre-built *clones* of a real 5★ operator, re-imported as a separate 6★ entry with the identical name (Sharp, Pith, Stormeye, Touch, Tulip, Shalem, Raidian) — were not, inflating the loader's reported total to 447 and leaving 7 pure duplicate entries in the pool. Meanwhile the tier list (which looks operators up by name) already collapsed duplicate names down to 437, so the two numbers never agreed and neither matched the true roster size.
- **Fix**: `is_excluded_operator()` now also excludes any entry whose `char_id` falls in the `608`-`614` numeric range, which cleanly and exclusively identifies all 7 Elite Operator clones (verified by cross-referencing raw game data: each clone shares an identical `subProfessionId` with its real counterpart). This was verified against two look-alike cases that must **not** be excluded: Amiya's 3 entries (Caster/Guard-Warrior/Medic) are all genuinely distinct alternate combat forms, and Mechanist/Raidian each have one real CN-exclusive operator that happens to share an English appellation with an unrelated Elite-Operator clone — distinguished because the real operators have a *different* `subProfessionId` from the clone. The homepage and comparisons operator lists also now dedupe by name (matching how the tier list's name-keyed lookup already behaves, and how the game itself treats Amiya as one roster slot with alternate forms), so the loader, homepage, and tier list now consistently report **437** operators.

### ✨ New: Rarity/Class/Archetype filters and class/archetype icons on the tier list
- Added checkbox filters for Rarity, Class, and Archetype to the tier list; checking a class auto-checks all of its archetypes.
- Added real class and archetype icons (sourced from the myrtle.moe asset API, credited in the README acknowledgements) next to each operator's photo on tier list cards — positioned alongside the portrait rather than overlaid on top of it, so the operator art stays unobstructed. Archetype names are shown in English rather than the raw Chinese subclass identifiers.

### 🏠 New: Homepage / Direct Comparisons split
- The former single Dashboard page mixed a project overview with the operator-vs-operator comparison tool. Split into two: `/` is now a proper homepage (roster breakdown by rarity/class, quick links to the Tier List, Enemy Tier List, Direct Comparisons, and the Operator/Enemy Editors), and the comparison tool moved to its own `/compare` route (**Direct Comparisons**).

### ✨ New: Teams page — Team Builder + auto-generated Team Tier List
Arknights is played with 8-12 deployed operators at once, not one at a time — this is the engine's first pass at scoring a *team* rather than an individual. New `/teams` page with two tabs, built as an aggregation layer on top of the existing per-operator simulation (no new combat math): every number below is read straight out of the same `evaluate_single_operator` output the individual Tier List already computes.

#### Team Builder tab
- Manually assemble a 12-operator squad (rarity/class/archetype filters + per-operator exclude toggle + a client-side Randomizer respecting both), and get an instant 6-axis radar: **Boss Killing**, **Lane Holding**, **Resistance**, **Utility**, **Consistency**, **Reliability** — each graded A-E, plus an overall score/grade and the top contributors per axis.
- Rule-based **recommendations** call out concrete gaps ("add a dedicated healer", "add a DP-generating Vanguard", "this team over-specializes — diversify roles") instead of just a number.
- `POST /api/team_score` computes this live per request (~36 `evaluate_single_operator` combo-searches for 12 operators — far cheaper than the full-roster `/api/tierlist_data` scan), so there's no caching to go stale.
- **Loadout picker**: click any team member to lock in a specific skill/module instead of the auto-picked best-`total_score` config — `total_score` has no notion of uptime/cooldown at all, so for an operator whose strongest skill has a much worse duration-to-cost ratio than a weaker one (e.g. Pramanix the Prerita's S3 vs. S2), the auto-pick could systematically understate Consistency for a real, deliberate loadout choice. The picker is an **inline panel** in the Team column (not a blocking modal) so switching which slot you're editing, or bouncing back to the roster, never requires closing anything first; each tile click re-scores the team immediately. Skill/module tiles show real in-game icons, fetched live client-side from myrtle.moe's public API by the operator's `char_id` (our own dataset has no skill IDs, and a module's icon ID is a different string than the equip ID we store) — falls back to text tags (S1/S2/S3 + SP cost, module code) if the fetch fails or is offline.

#### Team Tier List tab — an auto-generated ranking of team compositions
- Full combinatorics is astronomically infeasible (C(437,12) is on the order of 10¹⁹), so this runs a genetic search instead: an initial random population of 12-operator teams is evolved over many generations toward high overall score (elitism + tournament selection + crossover + random "immigrants" each generation to avoid collapsing onto one repeated "super team"), tracking every distinct composition seen as a "hall of fame". Cached in memory (`GET /api/team_tierlist`, `POST /api/team_tierlist/recalculate`) and **auto-invalidates** the next time the page is opened after the roster data changes (new/edited operators) — no manual action needed.
- **Candidate pool**: each class's top 5 individually-ranked operators (same per-operator `score` the main Tier List ranks by) — roughly 40 operators. Dense enough for the genetic search to explore thoroughly (still not exhaustive: C(40,12) ≈ 5.6 billion), at the cost of excluding anyone outside their class's top 5 from the *auto-generated* list specifically (the Team Builder above has no such restriction). An earlier version gated on "B-tier-or-better individually" (~214 operators) instead of a per-class cutoff, and before that tried scoring raw stats directly — both are noted below since they surfaced real scoring bugs before landing on this approach.
- Results are capped at the top 500 by score (a full run finds far more distinct teams than that, especially against a larger pool); the operator-name filter above the list plus paginated "Load More" rendering keep the page responsive at that size.

#### Bugs found and fixed while building the composite score
- **Vanguard-stacking exploit**: weight-averaging DP cost by "does this operator matter" can only ever be pulled *down* by adding another cheap operator, never up — so the genetic search had a standing incentive to pad slots with extra cheap Vanguards regardless of whether the team needed them (one generated "OP" team ran **6 Vanguards** out of 12). Fixed two ways: (1) deploy speed is now a minor 20% slice of the Utility axis instead of 45%, and (2) a direct **role-balance penalty** multiplies the overall score down when any single class exceeds 3 of the 12 slots (25% overall-score penalty per excess member) — this is the fix that actually stopped the stacking, since Boss Killing/Lane Holding already gate to their top 3-4 real contributors and never penalized redundant copies riding along for free.
- **Healing judged in isolation from the team's HP pool**: Resistance originally added a flat `total_heal` number, so a single real healer could push a 12-operator team to a 97-99 Resistance score regardless of how much total EHP that team actually had to sustain. Now healing is judged as `total_heal / total_team_EHP_pool` — the same absolute heal number means a lot for a squishy core and very little for a team with a huge combined health pool.
- **`efficiency` defaulting to 100 for zero-impact filler**: skill-less 1★ "gadget" operators (no skill = no cooldown = trivially "100% uptime") could single-handedly drag a real team's Consistency average to 100 despite contributing nothing else. Consistency (and the Utility axis's deploy-cost average) are now weighted by a rough `impact_weight` per operator (combat/heal/block/support contribution), so near-zero-impact operators barely move either.
- **Candidate pool couldn't find genuine specialists**: an earlier "top-N per raw stat" pool (top boss-damage, top survivability, top heal, etc.) meant a pure CC/debuff kit like Lappland the Decadenza — individually **OP**-tier — never topped any single raw-stat bucket and could never appear in the auto-generated list at all, even via the operator-name filter. Replaced with the tier-based and then class-based gates described above, both keyed off the same individual ranking already shown elsewhere in the app.

### ✨ New: Data Export — CSV replaces PDF/ZIP
The Tier List's **"DESCARGAR PDFS (.ZIP)"** button generated a ~22 MB archive of 116 pre-rendered PDF reports server-side (and could spawn a Python subprocess to regenerate it) — heavy for what a spreadsheet already covers. Removed the button and its `/api/tierlist/export_pdfs_zip` endpoint entirely; **EXPORT CSV** (already present, already covers the same ranks/tiers/scores/damage/survivability data) is now the sole export path. The standalone `Scripts/generate_tierlist_pdfs.py` generator still exists for anyone who wants offline PDF reports — it's just no longer wired into the web app.

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

// Shared class/archetype icon + display-name helpers, used by the Tier List and Team Builder
// pages. Kept in one place so both stay in sync instead of duplicating this ~150-line block.

const CLASS_ICON_FILE = {
    'PIONEER': 'pioneer', 'WARRIOR': 'warrior', 'TANK': 'tank', 'SNIPER': 'sniper',
    'CASTER': 'caster', 'SUPPORT': 'support', 'MEDIC': 'medic', 'SPECIAL': 'special'
};
const CLASS_LABEL = {
    'PIONEER': 'Vanguard', 'WARRIOR': 'Guard', 'TANK': 'Defender', 'SNIPER': 'Sniper',
    'CASTER': 'Caster', 'SUPPORT': 'Supporter', 'MEDIC': 'Medic', 'SPECIAL': 'Specialist'
};
function classIconUrl(profession) {
    const file = CLASS_ICON_FILE[profession];
    return file ? `/static/images/classes/${file}.png` : '';
}

// English archetype names, keyed by `sub_profession_id` (e.g. "duelist", "physician") —
// sourced from the game's own uniequip_table SubProfessionName strings, not the Chinese
// `subclass_name` text some operators (mostly recent CN-only imports) carry instead.
const SUB_PROFESSION_NAMES = {
    agent: "Agent", alchemist: "Alchemist", aoesniper: "Artilleryman", artsfghter: "Arts Fighter",
    artsprotector: "Arts Protector", bard: "Bard", bearer: "Standard Bearer", blastcaster: "Blast Caster",
    blessing: "Abjurer", bombarder: "Flinger", centurion: "Centurion", chain: "Chain Caster",
    chainhealer: "Chain Medic", charger: "Charger", closerange: "Heavyshooter", corecaster: "Core Caster",
    counsellor: "Strategist", craftsman: "Artificer", crusher: "Crusher", dollkeeper: "Dollkeeper",
    duelist: "Duelist", executor: "Executor", fastshot: "Marksman", fearless: "Dreadnought",
    fighter: "Fighter", fortress: "Fortress", funnel: "Mech-accord Caster", geek: "Geek",
    guardian: "Guardian", hammer: "Earthshaker", healer: "Therapist", hookmaster: "Hookmaster",
    hunter: "Hunter", incantationmedic: "Incantation Medic", instructor: "Instructor",
    librator: "Liberator", longrange: "Deadeye", loopshooter: "Loopshooter", lord: "Lord",
    mercenary: "Mercenary", merchant: "Merchant", musha: "Soloblade", mystic: "Mystic Caster",
    phalanx: "Phalanx Caster", physician: "Medic", pioneer: "Pioneer", primcaster: "Primal Caster",
    primguard: "Primal Guard", primprotector: "Primal Protector", protector: "Protector",
    pusher: "Push Stroker", reaper: "Reaper", reaperrange: "Spreadshooter", ringhealer: "Multi-target Medic",
    ritualist: "Ritualist", shotprotector: "Sentry Protector", siegesniper: "Besieger",
    skybreaker: "Skybreaker", skywalker: "Skyranger", slower: "Decel Binder", soulcaster: "Shaper Caster",
    splashcaster: "Splash Caster", stalker: "Ambusher", summoner: "Summoner",
    supportiveranger: "Supportive Ranger", sword: "Swordmaster", tactician: "Tactician",
    traper: "Trapmaster", underminer: "Hexer", unyield: "Juggernaut", wandermedic: "Wandering Medic",
    watchman: "Watchman Medic"
};
function archetypeName(subProfessionId, fallback) {
    return SUB_PROFESSION_NAMES[subProfessionId] || fallback || subProfessionId || 'Unknown';
}
function archetypeIconUrl(subProfessionId) {
    return subProfessionId ? `/static/images/archetypes/${subProfessionId}.png` : '';
}

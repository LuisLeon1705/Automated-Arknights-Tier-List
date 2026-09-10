#[path = "../core/mod.rs"]
mod core;

fn main() {
    let loader = core::data_loader::DataLoader::new("../data");
    let mut op = loader.get_operator("Kazemaru").unwrap();
    let s1 = op.skills[0].clone();
    println!("S1: {:?}", s1);
    op.equipped_skill = Some(s1);
    op.change_state(Some("0".to_string()));
    
    let mut target_stats = std::collections::HashMap::new();
    target_stats.insert("def".to_string(), 250.0);
    target_stats.insert("res".to_string(), 20.0);
    target_stats.insert("atk".to_string(), 500.0);
    target_stats.insert("attack_interval".to_string(), 3.0);
    
    let mut env = core::simulation::SimulationEnvironment::new(op, None, Some(target_stats));
    let rates = env.state_rates();
    println!("Rates: phys={:.2}, arts={:.2}, true={:.2}", rates.phys_per_shot, rates.arts_per_shot, rates.true_per_shot);
    let (cycle, burst) = env.cycle_at(250.0, 20.0);
    println!("cycle_at: {:?}, burst={:.2}", cycle, burst);
}

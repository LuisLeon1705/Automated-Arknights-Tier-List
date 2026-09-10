#![allow(unused_variables)]
#[path = "../core/mod.rs"]
mod core;

fn main() {
    let loader = core::data_loader::DataLoader::new("../data");
    let avg_enemy = core::enemy::calculate_average_enemy("../data");

    // Test Sakiko Togawa
    if let Some(op) = loader.get_operator("Sakiko Togawa") {
        println!("Sakiko Togawa found: {} skills", op.skills.len());
        for (si, s) in op.skills.iter().enumerate() {
            println!("  S{}: {}", si+1, s.name);
        }
    } else {
        println!("Sakiko Togawa not found");
    }

    // Test Ch'en the Dawnstreak
    if let Some(op) = loader.get_operator("Ch'en the Dawnstreak") {
        println!("Ch'en the Dawnstreak found: {} skills", op.skills.len());
        for (si, s) in op.skills.iter().enumerate() {
            println!("  S{}: {}", si+1, s.name);
        }
    } else {
        println!("Ch'en the Dawnstreak not found");
    }
}

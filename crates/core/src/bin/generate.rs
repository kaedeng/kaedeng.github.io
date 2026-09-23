fn main() {
    let Some(id) = std::env::args().nth(1) else {
        eprintln!("usage: generate <seed>[-easy|-hard]");
        std::process::exit(2);
    };
    let puzzle = patches_core::generate(&id);
    println!(
        "{}",
        serde_json::to_string_pretty(&puzzle).expect("puzzle serialises")
    );
}

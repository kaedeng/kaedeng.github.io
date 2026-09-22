fn main() {
    let Some(seed) = std::env::args().nth(1) else {
        eprintln!("usage: generate <seed>");
        std::process::exit(2);
    };
    let puzzle = patches_core::generate(&seed);
    println!(
        "{}",
        serde_json::to_string_pretty(&puzzle).expect("puzzle serialises")
    );
}

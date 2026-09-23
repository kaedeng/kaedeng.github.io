fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let puzzle = match args.as_slice() {
        [flag, date] if flag == "--daily" => patches_core::daily(date),
        [id] => patches_core::generate(id),
        _ => {
            eprintln!("usage: generate <seed>[-easy|-hard]\n       generate --daily <yyyy-mm-dd>");
            std::process::exit(2);
        }
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&puzzle).expect("puzzle serialises")
    );
}

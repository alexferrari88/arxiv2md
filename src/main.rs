fn main() {
    if let Err(error) = arxiv2md::run() {
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}

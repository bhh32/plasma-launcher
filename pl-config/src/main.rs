use pl_config::Config;

fn main() {
    if let Err(e) = Config::write_default_if_missing() {
        eprintln!("{e}");
    }

    let config = match Config::load() {
        Ok(config) => config,
        Err(e) => {
            eprintln!("{e}; using defaults");
            Config::default()
        }
    };

    match config.to_json() {
        Ok(json) => println!("{json}"),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1)
        }
    }
}

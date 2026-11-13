use std::io::{self, Write};

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("--version" | "-V") => {
            println!("starship-worktrunk {}", env!("CARGO_PKG_VERSION"));
            return;
        }
        Some("--help" | "-h") => {
            println!(
                "Render Starship's directory module with Worktrunk branch suffixes removed.\n\n\
                 Usage: starship-worktrunk [--version | --help]"
            );
            return;
        }
        Some(argument) => {
            eprintln!("starship-worktrunk: unexpected argument: {argument}");
            std::process::exit(2);
        }
        None => {}
    }
    if let Err(error) =
        starship_worktrunk::render().and_then(|output| io::stdout().lock().write_all(&output))
    {
        eprintln!("starship-worktrunk: {error}");
        std::process::exit(1);
    }
}

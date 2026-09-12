use std::io::{self, Write};

fn main() {
    if let Err(error) =
        starship_worktrunk::render().and_then(|output| io::stdout().lock().write_all(&output))
    {
        eprintln!("starship-worktrunk: {error}");
        std::process::exit(1);
    }
}

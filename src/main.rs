fn main() {
    let result = starship_worktrunk::render();
    for warning in result.warnings {
        eprintln!("starship-worktrunk: warning: {warning}");
    }
    println!("{}", result.output);
}

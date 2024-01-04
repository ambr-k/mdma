use std::process::Command;

fn main() {
    run_tailwind();
}

fn run_tailwind() {
    let output = Command::new("npx")
        .args([
            "tailwindcss",
            "-i",
            "templates/styles.css",
            "-o",
            "static/styles.css",
        ])
        .output()
        .unwrap();
    if !output.status.success() {
        panic!("{}", String::from_utf8(output.stderr).unwrap());
    }
}

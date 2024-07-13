use std::process::Command;

fn main() {
    run_tailwind();
}

fn run_tailwind() {
    let result = Command::new("npx")
        .args(["tailwindcss", "-i", "styles.css", "-o", "static/styles.css"])
        .output();
    match result {
        Ok(output) => {
            if !output.status.success() {
                panic!("{}", String::from_utf8(output.stderr).unwrap());
            }
        }
        Err(err) => {
            println!("[WARN] tailwind failed: {}", err.to_string());
        }
    }
}

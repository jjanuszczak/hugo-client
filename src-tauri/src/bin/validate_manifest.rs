use std::{env, fs, process};

fn main() {
    let Some(path) = env::args_os().nth(1) else {
        eprintln!("Usage: validate_manifest PATH_TO_MANIFEST_JSON");
        process::exit(2);
    };
    let manifest_text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) => {
            eprintln!("Cannot read manifest: {error}");
            process::exit(2);
        }
    };
    let manifest: serde_json::Value = match serde_json::from_str(&manifest_text) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("Manifest JSON is invalid: {error}");
            process::exit(1);
        }
    };
    match hugo_native_client_lib::validate_manifest_v1(&manifest) {
        Ok(()) => println!("Manifest v1 is valid."),
        Err(error) => {
            eprintln!("{error}");
            process::exit(1);
        }
    }
}

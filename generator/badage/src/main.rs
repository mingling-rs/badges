use std::{
    env::current_dir,
    io::{Error, ErrorKind::Other},
    path::{Path, PathBuf},
    process::exit,
};

use arg_picker::{macros::arg, Picker, PickerArg};
use just_fmt::kebab_case;

const KEY: PickerArg<String> = arg![key: _];
const VALUE: PickerArg<String> = arg![value: _];

/// USAGE: mingling-badge-gen --key <KEY> --value <VALUE>
///
/// Example: mingling-badge-gen --key version --value "0.1.0"
/// > Badage generated ==> ./badage-key.png
fn main() {
    let (badage_key, badage_value) = Picker::from_args()
        .pick_or(&KEY, || "Unknown".into())
        .pick_or(&VALUE, || "Unknown".into())
        .unwrap();

    let export_file = export_file(&badage_key);

    if export_file.exists() {
        eprintln!(
            "Error: file {} founded, please try other name. (e.g. {}_new)",
            export_file.to_string_lossy(),
            export_file
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
        );
        exit(2)
    }

    match generate(&badage_key, &badage_value, &export_file) {
        Ok(_) => eprintln!(
            "Badage generated ==> \"{}\"",
            &export_file.to_string_lossy()
        ),
        Err(e) => match e.kind() {
            Other => {
                eprintln!("Generate failed: {}", e.to_string());
                exit(1)
            }
            _ => {
                eprintln!("Generate failed (IOError): {}", e.to_string());
                exit(3)
            }
        },
    }
}

/// Exports the file with a kebab-cased badge name in the current directory.
fn export_file(name: &String) -> PathBuf {
    current_dir()
        .unwrap()
        .join(format!("badage-{}", kebab_case!(name)))
}

/// Generates a badge image file.
///
/// # Arguments
///
/// * `key` - The badge key text.
/// * `value` - The badge value text.
/// * `out_file` - The output file path for the generated badge.
///
/// # Returns
///
/// `Ok(())` on success, or an `Error` if the badge generation fails.
fn generate(key: &String, value: &String, out_file: &Path) -> Result<(), Error> {
    Ok(())
}

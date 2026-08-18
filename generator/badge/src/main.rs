use std::{env::current_dir, io::ErrorKind::Other, path::PathBuf, process::exit};

use arg_picker::{Picker, PickerArg, macros::arg};
use just_fmt::kebab_case;

mod badge;
use badge::generate;

const KEY: PickerArg<String> = arg![key: _];
const VALUE: PickerArg<String> = arg![value: _];

/// USAGE: mingling-badge-gen --key <KEY> --value <VALUE>
///
/// Example: mingling-badge-gen --key version --value "0.1.0"
/// > badge generated ==> ./badge-key.png
fn main() {
    let (badge_key, badge_value) = Picker::from_args()
        .pick_or(&KEY, || "Unknown".into())
        .pick_or(&VALUE, || "Unknown".into())
        .unwrap();

    let export_file = export_file(&badge_key);

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

    match generate(&badge_key, &badge_value, &export_file) {
        Ok(_) => eprintln!("badge generated ==> \"{}\"", &export_file.to_string_lossy()),
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
        .join(format!("badge-{}.png", kebab_case!(name)))
}

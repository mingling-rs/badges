use std::{env::current_dir, io::ErrorKind::Other, path::PathBuf, process::exit};

use arg_picker::{Picker, PickerArg, macros::arg};
use just_fmt::kebab_case;

mod badge;
use badge::{BadgeStyle, Color, generate_with_style};

const KEY: PickerArg<String> = arg![key: _];
const VALUE: PickerArg<String> = arg![value: _];
const BORDER_WIDTH: PickerArg<f32> = arg![border_width: _];
const BORDER_COLOR: PickerArg<Color> = arg![border_color: _];
const BORDER_RADIUS: PickerArg<f32> = arg![border_radius: _];
const KEY_BG: PickerArg<Color> = arg![key_background_color: _];
const VALUE_BG: PickerArg<Color> = arg![value_background_color: _];
const KEY_FG: PickerArg<Color> = arg![key_font_color: _];
const VALUE_FG: PickerArg<Color> = arg![value_font_color: _];
const PADDING: PickerArg<f32> = arg![padding: _];

/// USAGE: mingling-badge-gen --key <KEY> --value <VALUE> [style options]
///
/// Example:
///   mingling-badge-gen --key version --value "0.1.0"
///   mingling-badge-gen --key version --value "0.1.0" \
///       --border-color "#d4a84b" --border-radius 6 --padding 20
///
/// Style options (geometry values are in 30px-design units, scaled up to
/// the 256px-tall output; HEX accepts "#RRGGBB" or "#AARRGGBB"):
///   --border-width <f32>            default 1
///   --border-color <HEX>            default #3a2e24
///   --border-radius <f32>           default 3
///   --key-background-color <HEX>    default #241c16
///   --value-background-color <HEX>  default #d4a84b
///   --key-font-color <HEX>          default #e8ddd0
///   --value-font-color <HEX>        default #1a1410
///   --padding <f32>                 default 12
fn main() {
    let (
        badge_key,
        badge_value,
        border_width,
        border_color,
        border_radius,
        key_background,
        value_background,
        key_font,
        value_font,
        padding,
    ) = Picker::from_args()
        .pick_or(&KEY, || "Unknown".into())
        .pick_or(&VALUE, || "Unknown".into())
        .pick_or(&BORDER_WIDTH, || BadgeStyle::default().border_width)
        .pick_or(&BORDER_COLOR, || BadgeStyle::default().border_color)
        .pick_or(&BORDER_RADIUS, || BadgeStyle::default().border_radius)
        .pick_or(&KEY_BG, || BadgeStyle::default().key_bg)
        .pick_or(&VALUE_BG, || BadgeStyle::default().value_bg)
        .pick_or(&KEY_FG, || BadgeStyle::default().key_fg)
        .pick_or(&VALUE_FG, || BadgeStyle::default().value_fg)
        .pick_or(&PADDING, || BadgeStyle::default().padding)
        .unwrap();

    let style = BadgeStyle {
        border_width,
        border_color,
        border_radius,
        key_bg: key_background,
        value_bg: value_background,
        key_fg: key_font,
        value_fg: value_font,
        padding,
    };

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

    match generate_with_style(&badge_key, &badge_value, &export_file, &style) {
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

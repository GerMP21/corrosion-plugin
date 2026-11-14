//! Writes the user guide's data (`web/guide/src/data/*.json`) from the
//! plugin's parameters, control tables and models (`add-user-guide`).
//!
//! Run after changing a parameter, a label or ⓘ text, a model or the
//! factory bank: `cargo run --bin export_guide_data`.
//! `guide_data_equals_the_files_on_disk` fails until it is rerun.

fn main() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    for (path, contents) in corrosion::guide::guide_files() {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().expect("a data folder")).expect("create the folder");
        std::fs::write(&path, contents).expect("write the file");
        println!("wrote {}", path.display());
    }
}

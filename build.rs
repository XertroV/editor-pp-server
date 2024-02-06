// fn main() -> io::Result<()> {
//     if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
//         WindowsResource::new()
//         .set_windres_path("/home/xertrov/.zeranoe/mingw-w64/x86_64/bin/x86_64-w64-mingw32-windres")
//             // This path can be absolute, or relative to your crate root.
//             .set_icon("assets/icon.ico")
//             .compile()?;
//     }
//     Ok(())
// }
// build.rs
use std::env;
fn main() {
    if env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        use std::{
            fs::{copy, write},
            path::PathBuf,
            process::Command,
        };
        let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
        let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
        copy(manifest_dir.join("assets/icon.ico"), out_dir.join("icon.ico")).unwrap();
        copy(manifest_dir.join("icon.rc"), out_dir.join("icon.rc")).unwrap();
        // write(out_dir.join("icon.rc"), "icon ICON icon.ico").unwrap();
        Command::new("x86_64-w64-mingw32-windres")
            .current_dir(&out_dir)
            .arg("icon.rc")
            .arg("icon.lib")
            .spawn()
            .unwrap();
        println!(
            "cargo:rustc-link-search={}",
            out_dir.into_os_string().into_string().unwrap()
        );
        println!("cargo:rustc-link-lib=icon");
    }
}

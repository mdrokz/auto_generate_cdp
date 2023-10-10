use std::env;
use std::path::Path;
use std::fs::File;
use std::io::Write;

fn main() {
    if cfg!(feature = "offline") {
        let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    
        #[cfg(target_os = "windows")]
        let manifest_dir = manifest_dir.replace(r#"\"#,r#"\\"#);
    
        let out_dir = env::var("OUT_DIR").unwrap();
        let dest_path = Path::new(&out_dir).join("path.rs");
    
        let mut f = File::create(&dest_path).unwrap();
        write!(
            f,
            "pub const MANIFEST_DIR: &str = \"{}\";",
            manifest_dir
        ).unwrap();
    }
}
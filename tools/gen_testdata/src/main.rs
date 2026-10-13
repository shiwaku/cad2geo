//! 合成の図面を書き出す。既定の出力先は testdata/synthetic（リポジトリに入れる）。
//!
//! cargo run -p cad2geojson-gen-testdata [-- <dir>]

use std::path::PathBuf;

fn main() -> std::io::Result<()> {
    let dir = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/synthetic")
        });
    std::fs::create_dir_all(&dir)?;
    for (name, bytes) in cad2geojson_gen_testdata::files() {
        let p = dir.join(name);
        std::fs::write(&p, &bytes)?;
        eprintln!("wrote {} ({} bytes)", p.display(), bytes.len());
    }
    Ok(())
}

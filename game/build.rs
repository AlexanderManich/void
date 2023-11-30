use rand::Rng;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;

fn main() {
    // This is a hack to prevent the debugger from locking the pdb file when hot reloading the library
    for path in fs::read_dir(dbg!(Path::new("../target/debug/deps"))).unwrap() {
        let path = path.unwrap().path();
        if let Some(extension) = path.extension() {
            if extension == OsStr::new("pdb") {
                fs::remove_file(path).unwrap();
            }
        };
    }

    let mut rng = rand::thread_rng();

    let pdb_name: String = format!("game_{}.pdb", rng.gen_range(0..10));
    println!("cargo:rustc-link-arg=/PDB:target/debug/deps/{}", pdb_name);
}

fn main() {
    let ts_core = env!("CARGO_PKG_VERSION");

    let lang_versions: Vec<(&str, &str)> = vec![
        #[cfg(feature = "core-languages")]
        ("tree-sitter-rust", "0.21.0"),
        #[cfg(feature = "core-languages")]
        ("tree-sitter-python", "0.21.0"),
        #[cfg(feature = "core-languages")]
        ("tree-sitter-javascript", "0.21.0"),
        #[cfg(feature = "core-languages")]
        ("tree-sitter-typescript", "0.21.0"),
    ];

    for (name, expected) in &lang_versions {
        let major_core = ts_core.split('.').next().unwrap_or("0");
        let major_lang = expected.split('.').next().unwrap_or("0");
        if major_core != major_lang {
            panic!(
                "FATAL: tree-sitter version mismatch! \
                 core={} but {}={}. \
                 Major version must be identical to prevent ABI segfault. \
                 Fix in [workspace.dependencies] of Cargo.toml.",
                ts_core, name, expected
            );
        }
    }

    println!("cargo:rerun-if-changed=Cargo.toml");
}

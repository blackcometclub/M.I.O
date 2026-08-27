fn main() {
    #[cfg(windows)]
    {
        let manifest = std::path::Path::new("windows-common-controls-v6.manifest")
            .canonicalize()
            .expect("Windows Common Controls manifest must exist");

        println!("cargo:rerun-if-changed={}", manifest.display());
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());

        let windows = tauri_build::WindowsAttributes::new_without_app_manifest();
        let attributes = tauri_build::Attributes::new().windows_attributes(windows);
        tauri_build::try_build(attributes).expect("failed to run Tauri build script");
    }

    #[cfg(not(windows))]
    tauri_build::build();
}

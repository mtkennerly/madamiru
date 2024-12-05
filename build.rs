#[cfg(docsrs)]
fn main() {}

#[cfg(not(docsrs))]
fn main() {
    println!("cargo:rerun-if-env-changed=MADAMIRU_VERSION");
    println!("cargo:rerun-if-changed=assets/windows/manifest.xml");

    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set_manifest_file("assets/windows/manifest.xml");
        res.compile().unwrap();
    }

    // https://gitlab.freedesktop.org/gstreamer/gstreamer-rs/-/merge_requests/1516
    #[cfg(target_os = "macos")]
    match system_deps::Config::new().probe() {
        Ok(deps) => {
            let usr = std::path::Path::new("/usr/lib");
            let usr_local = std::path::Path::new("/usr/local/lib");
            for dep in deps.all_link_paths() {
                if dep != &usr && dep != &usr_local {
                    println!("cargo:rustc-link-arg=-Wl,-rpath,{:?}", dep.as_os_str());
                }
            }
        }
        Err(s) => {
            println!("cargo:warning={s}");
            std::process::exit(1);
        }
    }
}

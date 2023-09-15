use std::path::Path;
use std::process::*;
use std::{env, fs};

fn join(root: &str, next: &str) -> String {
    Path::new(root).join(next).to_str().unwrap().to_string()
}

#[cfg(target_os = "windows")]
fn exec(cmd: &str, work_dir: &str) -> Result<ExitStatus, std::io::Error> {
    Command::new("powershell")
        .args(["-command", cmd])
        .current_dir(work_dir)
        .status()
}

#[cfg(target_os = "windows")]
fn download_cef(out_dir: &str, cef_version: &str) {
    let cef_path = join(&out_dir, "./cef");
    if fs::metadata(join(&out_dir, "./cef.tar.bz2")).is_err() {
        exec(
            &format!(
                "Invoke-WebRequest -Uri https://cef-builds.spotifycdn.com/{}_windows64_minimal.tar.bz2 -OutFile cef.tar.bz2",
                cef_version
            ),
            out_dir,
        )
        .unwrap();
    }

    if fs::metadata(&cef_path).is_err() {
        let path = format!("./{}_windows64_minimal", cef_version);
        if fs::metadata(join(&out_dir, &path)).is_err() {
            exec("tar -xf ./cef.tar.bz2 -C ./", out_dir).unwrap();
        }

        exec(&format!("Rename-Item {} ./cef", path), out_dir).unwrap();
    }

    if fs::metadata(join(&cef_path, "./libcef_dll_wrapper")).is_err() {
        exec("cmake -DCMAKE_BUILD_TYPE=Release .", &cef_path).unwrap();
        exec("cmake --build . --config Release", &cef_path).unwrap();
    }
}

fn static_link(out_dir: &str, target: &str) {
    let mut cfgs = cc::Build::new();
    let is_debug = env::var("DEBUG")
        .map(|label| label == "true")
        .unwrap_or(true);

    cfgs.cpp(true)
        .debug(is_debug)
        .static_crt(true)
        .target(target)
        .warnings(false)
        .out_dir(out_dir);

    if cfg!(target_os = "windows") {
        cfgs.flag("/std:c++20");
    } else {
        cfgs.flag("-std=c++20");
    }

    cfgs.file("./package/src/app.cpp")
        .file("./package/src/browser.cpp")
        .file("./package/src/control.cpp")
        .file("./package/src/bridge.cpp")
        .file("./package/src/render.cpp")
        .file("./package/src/display.cpp")
        .file("./package/src/webview.cpp")
        .file("./package/src/scheme_handler.cpp")
        .file("./package/src/message_router.cpp");

    cfgs.include(join(out_dir, "./cef"));

    #[cfg(target_os = "windows")]
    cfgs.define("WIN32", Some("1"))
        .define("_WINDOWS", None)
        .define("__STDC_CONSTANT_MACROS", None)
        .define("__STDC_FORMAT_MACROS", None)
        .define("_WIN32", None)
        .define("UNICODE", None)
        .define("_UNICODE", None)
        .define("WINVER", Some("0x0A00"))
        .define("_WIN32_WINNT", Some("0x0A00"))
        .define("NTDDI_VERSION", Some("NTDDI_WIN10_FE"))
        .define("NOMINMAX", None)
        .define("WIN32_LEAN_AND_MEAN", None)
        .define("_HAS_EXCEPTIONS", Some("0"))
        .define("PSAPI_VERSION", Some("1"))
        .define("CEF_USE_SANDBOX", None)
        .define("CEF_USE_ATL", None)
        .define("_HAS_ITERATOR_DEBUGGING", Some("0"));

    #[cfg(target_os = "linux")]
    cfgs.define("LINUX", Some("1")).define("CEF_X11", Some("1"));

    cfgs.compile("package");

    println!(
        "cargo:rustc-link-search=all={}",
        join(out_dir, "./cef/libcef_dll_wrapper/Release")
    );
    println!(
        "cargo:rustc-link-search=all={}",
        join(out_dir, "./cef/Release")
    );
    println!("cargo:rustc-link-search=all={}", out_dir);
    println!("cargo:rustc-link-lib=static=package");

    #[cfg(target_os = "windows")]
    {
        println!("cargo:rustc-link-lib=libcef");
        println!("cargo:rustc-link-lib=libcef_dll_wrapper");
        println!("cargo:rustc-link-lib=delayimp");
        println!("cargo:rustc-link-lib=winmm");
        println!("cargo:rustc-link-arg=/NODEFAULTLIB:libcmt.lib")
    }

    #[cfg(target_os = "linux")]
    {
        println!("cargo:rustc-link-lib=cef");
        println!("cargo:rustc-link-lib=cef_dll_wrapper");
        println!("cargo:rustc-link-lib=X11");
    }
}

fn main() {
    let cef_version = "cef_binary_116.0.22+g480de66+chromium-116.0.5845.188";
    let target = env::var("TARGET").unwrap();
    let out_dir = env::var("OUT_DIR").unwrap();
    let temp = env::var("TEMP").unwrap();

    println!("cargo:rerun-if-changed=./package/src");
    println!("cargo:rerun-if-changed=./build.rs");
    println!("cargo:CEF_RELEASE={}", &join(&out_dir, "./cef"));

    download_cef(&out_dir, cef_version);
    static_link(&out_dir, &target);

    let temp_cef = join(&temp, &cef_version);
    if fs::metadata(&temp_cef).is_err() {
        fs::create_dir(&temp_cef).unwrap();
        exec(&format!("cp -r ./cef/Resources/* {}", &temp_cef), &out_dir).unwrap();
        exec(&format!("cp ./cef/Release/* {}", &temp_cef), &out_dir).unwrap();
    }
}

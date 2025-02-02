//
// Copyright (C) 2020 Jonas Zaddach.
//
// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 2 as
// published by the Free Software Foundation.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program; if not, write to the Free Software
// Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston,
// MA 02110-1301, USA.

use std::env;
use std::path::PathBuf;

// Generate bindings for these functions:
const BINDGEN_FUNCTIONS: &[&str] = &[
    "cli_ctx",
    "cli_warnmsg",
    "cli_dbgmsg_no_inline",
    "cli_infomsg_simple",
    "cli_errmsg",
    "cli_append_virus",
    "lsig_increment_subsig_match",
    "cli_versig2",
    "cli_getdsig",
    "cli_get_debug_flag",
    "cl_init",
    "cl_debug",
    "cl_engine_new",
    "cl_engine_get_num",
    "cl_engine_set_num",
    "cl_engine_get_str",
    "cl_engine_set_str",
    "cl_engine_settings_copy",
    "cl_engine_settings_apply",
    "cl_engine_settings_free",
    "cl_engine_compile",
    "cl_engine_addref",
    "cl_engine_free",
    "cl_engine_set_clcb_pre_cache",
    "cl_engine_set_clcb_pre_scan",
    "cl_engine_set_clcb_post_scan",
    "cl_engine_set_clcb_virus_found",
    "cl_engine_set_clcb_sigload",
    "cl_engine_set_clcb_sigload_progress",
    "cl_engine_set_clcb_engine_compile_progress",
    "cl_engine_set_clcb_engine_free_progress",
    "cl_set_clcb_msg",
    "cl_engine_set_clcb_hash",
    "cl_engine_set_clcb_meta",
    "cl_engine_set_clcb_file_props",
    "cl_engine_set_stats_set_cbdata",
    "cl_engine_set_clcb_stats_add_sample",
    "cl_engine_set_clcb_stats_remove_sample",
    "cl_engine_set_clcb_stats_decrement_count",
    "cl_engine_set_clcb_stats_submit",
    "cl_engine_set_clcb_stats_flush",
    "cl_engine_set_clcb_stats_get_num",
    "cl_engine_set_clcb_stats_get_size",
    "cl_engine_set_clcb_stats_get_hostid",
    "cl_engine_stats_enable",
    "cl_scandesc",
    "cl_scandesc_callback",
    "cl_scanfile",
    "cl_scanfile_callback",
    "cl_load",
    "cl_retdbdir",
    "cl_retflevel",
    "cl_retver",
    "cl_fmap_open_memory",
    "cl_fmap_close",
    "cl_scanmap_callback",
    "cl_strerror",
];

// Generate bindings for these types (structs, enums):
const BINDGEN_TYPES: &[&str] = &["cli_matcher", "cli_ac_data", "cli_ac_result"];

const BINDGEN_CONSTANTS: &[&str] = &[
    "CL_SCAN_.*",
    "CL_INIT_DEFAULT",
    "CL_DB_.*",
    "ENGINE_OPTIONS_.*",
];

fn generate_bindings(customize_bindings: &dyn Fn(bindgen::Builder) -> bindgen::Builder) {
    let mut bindings = bindgen::Builder::default();
    for function in BINDGEN_FUNCTIONS {
        bindings = bindings.whitelist_function(function);
    }

    for typename in BINDGEN_TYPES {
        bindings = bindings.whitelist_type(typename);
    }

    for constant in BINDGEN_CONSTANTS {
        bindings = bindings.whitelist_var(constant);
    }

    bindings = bindings
        .header("wrapper.h")
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks));

    // Prevent some constants from inappropriately receiving prefix of their type.
    //  Without this, e.g., CL_CLEAN becomes cl_error_t_CL_CLEAN
    bindings = bindings.prepend_enum_name(false);

    bindings = customize_bindings(bindings);

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    bindings
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings")
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}

fn cargo_common() {
    println!("cargo:rustc-link-lib=dylib={}", "clamav");

    // Tell cargo to invalidate the built crate whenever the wrapper changes
    println!("cargo:rerun-if-changed=wrapper.h");
}

#[cfg(windows)]
fn main() {
    let include_paths = match vcpkg::find_package("clamav") {
        Ok(pkg) => pkg.include_paths,
        Err(err) => {
            println!(
                "cargo:warning=Either vcpkg is not installed, or an error occurred in vcpkg: {}",
                err
            );
            let clamav_source = PathBuf::from(env::var("CLAMAV_SOURCE").expect("CLAMAV_SOURCE environment variable must be set and point to ClamAV's source directory"));
            let clamav_build = PathBuf::from(env::var("CLAMAV_BUILD").expect("CLAMAV_BUILD environment variable must be set and point to ClamAV's build directory"));
            let openssl_include = PathBuf::from(env::var("OPENSSL_INCLUDE").expect("OPENSSL_INCLUDE environment variable must be set and point to openssl's include directory"));
            let profile = env::var("PROFILE").unwrap();

            let library_path = match profile.as_str() {
                "debug" => std::path::Path::new(&clamav_build).join("libclamav/Debug"),
                "release" => std::path::Path::new(&clamav_build).join("libclamav/Release"),
                _ => panic!("Unexpected build profile"),
            };

            println!(
                "cargo:rustc-link-search=native={}",
                library_path.to_str().unwrap()
            );

            vec![
                clamav_source.join("libclamav"),
                clamav_build,
                openssl_include,
            ]
        }
    };

    cargo_common();
    generate_bindings(&|x: bindgen::Builder| -> bindgen::Builder {
        let mut x = x;
        for include_path in &include_paths {
            x = x.clang_arg("-I").clang_arg(include_path.to_str().unwrap());
        }
        x
    });
}

#[cfg(unix)]
fn main() {
    let libclamav = pkg_config::Config::new()
        .atleast_version("0.103")
        .probe("libclamav")
        .unwrap();

    let mut include_paths = libclamav.include_paths.clone();

    if let Some(val) = std::env::var_os("OPENSSL_ROOT_DIR") {
        let mut openssl_include_dir = PathBuf::from(val);
        openssl_include_dir.push("include");
        include_paths.push(openssl_include_dir);
    }

    cargo_common();
    generate_bindings(&|x: bindgen::Builder| -> bindgen::Builder {
        let mut x = x;
        for include_path in &include_paths {
            x = x.clang_arg("-I").clang_arg(include_path.to_str().unwrap());
        }
        x
    });
}

use bindgen::Builder;
use std::process::Command;
use std::path::Path;
use std::env;
use std::collections::{HashSet, BTreeSet};
use std::sync::{Arc, RwLock};
use bindgen::callbacks::{MacroParsingBehavior, ParseCallbacks};

// Taken from https://github.com/openebs/spdk-sys
#[derive(Clone, Debug)]
struct MacroCallback {
    macros: Arc<RwLock<HashSet<String>>>,
}

impl ParseCallbacks for MacroCallback {
    fn will_parse_macro(&self, name: &str) -> MacroParsingBehavior {
        self.macros.write().unwrap().insert(name.into());

        if name == "IPPORT_RESERVED" {
            return MacroParsingBehavior::Ignore;
        }

        MacroParsingBehavior::Default
    }
}


fn main() {
    println!("cargo:rerun-if-env-changed=PKG_CONFIG_PATH");

    let libs = &[
        "spdk_nvme",
        "spdk_sock",
        "spdk_sock_posix",
        "spdk_thread",
        "spdk_util",
        "spdk_vmd",
        "spdk_env_dpdk",
        "spdk_log",
    ];

    let mut header_locations = BTreeSet::new();

    for lib in libs {
        let cflags_bytes = Command::new("pkg-config")
            .args(&["--cflags", lib])
            .output()
            .unwrap_or_else(|e| panic!("Failed pkg-config cflags for {}: {:?}", lib, e))
            .stdout;
        let cflags = String::from_utf8(cflags_bytes).unwrap();

        for flag in cflags.split(' ') {
            if flag.starts_with("-I") {
                let header_location = flag[2..].trim();
                header_locations.insert(header_location.to_owned());
            } 
        }
    }

    let mut library_locations = BTreeSet::new();
    let mut lib_names = BTreeSet::new();

    for lib in libs {
        let ldflags_bytes = Command::new("pkg-config")
            .args(&["--libs", lib])
            .output()
            .unwrap_or_else(|e| panic!("Failed pkg-config ldflags for {}: {:?}", lib, e))
            .stdout;
        let ldflags = String::from_utf8(ldflags_bytes).unwrap();
        
        for flag in ldflags.split(' ') {
            if flag.starts_with("-L") {
                library_locations.insert(flag[2..].to_owned());
            } else if flag.starts_with("-l") {
                lib_names.insert(flag[2..].to_owned());
            }
        }
    }

    for library_location in &library_locations {
        println!("cargo:rustc-link-search={}", library_location);
    }
    for lib_name in &lib_names {
        println!("cargo:rustc-link-lib={}", lib_name);
    }
    println!("cargo:rustc-link-lib=spdk_env_dpdk");
    println!("cargo:rustc-link-lib=spdk_log");
    println!("cargo:rustc-link-lib=uuid");

    let mut builder = Builder::default();
    for header_location in &header_locations {
        println!("Including {}", header_location);
        builder = builder.clang_arg(&format!("-I{}", header_location));
    }

    let repr_align_errors = &[
        "spdk_nvme_tcp_cmd",
        "spdk_nvme_tcp_rsp",
        "spdk_nvmf_fabric_prop_get_rsp",
        "spdk_nvmf_fabric_connect_rsp",
        "spdk_nvmf_fabric_connect_cmd",
        "spdk_nvmf_fabric_auth_send_cmd",
        "spdk_nvmf_fabric_auth_recv_cmd",
        "spdk_nvme_health_information_page",
        "spdk_nvme_ctrlr_data",
        "spdk_nvme_sgl_descriptor",
    ];
    for item in repr_align_errors {
        builder = builder.opaque_type(item);
    }
    let macros = MacroCallback { macros: Arc::new(RwLock::new(HashSet::new())) };
    let bindings = builder 
        .header("wrapper.h")
        .rustfmt_bindings(true)
        .trust_clang_mangling(false)
        .layout_tests(false)
        .derive_default(true)
        .derive_debug(true)
        .prepend_enum_name(false)
        .generate_inline_functions(true)
        .parse_callbacks(Box::new(macros.clone()))
        .generate()
        .unwrap_or_else(|e| panic!("Failed to generate bindings: {:?}", e));
    let out_dir_s = env::var("OUT_DIR").unwrap();
    let out_dir = Path::new(&out_dir_s);
    let bindings_out = out_dir.join("bindings.rs");
    bindings.write_to_file(bindings_out).unwrap();
}

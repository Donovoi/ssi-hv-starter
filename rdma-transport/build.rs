//! Build script to generate RDMA bindings using bindgen
//!
//! This generates Rust FFI bindings from infiniband/verbs.h
//! Requires: libibverbs-dev package installed

use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // Check if we're in stub mode
    let stub_mode = env::var("CARGO_FEATURE_STUB_RDMA").is_ok();

    if !stub_mode {
        // Link against libibverbs only when not in stub mode
        println!("cargo:rustc-link-lib=ibverbs");
    }

    // Check if we should skip bindings generation (for CI/no-RDMA environments)
    if env::var("SSI_HV_SKIP_RDMA_BINDINGS").is_ok() || stub_mode {
        if stub_mode {
            println!("cargo:warning=Stub mode enabled - no RDMA bindings generated");
        } else {
            println!(
                "cargo:warning=Skipping RDMA bindings generation (SSI_HV_SKIP_RDMA_BINDINGS set)"
            );
        }
        return;
    } // Check if libibverbs-dev is available
    if !PathBuf::from("/usr/include/infiniband/verbs.h").exists() {
        println!("cargo:warning=libibverbs-dev not found, using stub implementation");
        println!("cargo:warning=Install with: sudo apt-get install libibverbs-dev");
        return;
    }

    // Generate bindings
    let bindings = bindgen::Builder::default()
        // Input header
        .header_wrapper()
        // Core verbs structures
        .allowlist_type("ibv_device")
        .allowlist_type("ibv_context")
        .allowlist_type("ibv_pd")
        .allowlist_type("ibv_mr")
        .allowlist_type("ibv_cq")
        .allowlist_type("ibv_qp")
        .allowlist_type("ibv_qp_init_attr")
        .allowlist_type("ibv_qp_attr")
        .allowlist_type("ibv_send_wr")
        .allowlist_type("ibv_recv_wr")
        .allowlist_type("ibv_sge")
        .allowlist_type("ibv_wc")
        .allowlist_type("ibv_port_attr")
        .allowlist_type("ibv_device_attr")
        .allowlist_type("ibv_gid")
        // Core functions
        .allowlist_function("ibv_get_device_list")
        .allowlist_function("ibv_free_device_list")
        .allowlist_function("ibv_get_device_name")
        .allowlist_function("ibv_open_device")
        .allowlist_function("ibv_close_device")
        .allowlist_function("ibv_query_device")
        .allowlist_function("ibv_query_port")
        .allowlist_function("ibv_query_gid")
        .allowlist_function("ibv_alloc_pd")
        .allowlist_function("ibv_dealloc_pd")
        .allowlist_function("ibv_reg_mr")
        .allowlist_function("ibv_dereg_mr")
        .allowlist_function("ibv_create_cq")
        .allowlist_function("ibv_destroy_cq")
        .allowlist_function("ibv_create_qp")
        .allowlist_function("ibv_destroy_qp")
        .allowlist_function("ibv_modify_qp")
        .allowlist_function("ibv_query_qp")
        .allowlist_function("ibv_post_send")
        .allowlist_function("ibv_post_recv")
        .allowlist_function("ibv_poll_cq")
        // Constants and enums
        .allowlist_var("IBV_QP_.*")
        .allowlist_var("IBV_ACCESS_.*")
        .allowlist_var("IBV_WR_.*")
        .allowlist_var("IBV_WC_.*")
        .allowlist_var("IBV_SEND_.*")
        .allowlist_var("IBV_MTU_.*")
        // QP types and states
        .allowlist_type("ibv_qp_type")
        .allowlist_type("ibv_qp_state")
        .allowlist_type("ibv_wr_opcode")
        .allowlist_type("ibv_wc_status")
        .allowlist_type("ibv_wc_opcode")
        .allowlist_type("ibv_access_flags")
        .allowlist_type("ibv_send_flags")
        .allowlist_type("ibv_qp_attr_mask")
        .allowlist_type("ibv_mtu")
        // Also allow _compat types
        .allowlist_type("_compat_.*")
        // Derive traits
        .derive_debug(true)
        .derive_default(true)
        .derive_copy(true)
        .derive_eq(true)
        .derive_hash(true)
        // Layout tests
        .layout_tests(true)
        // Generate the bindings
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings");

    // Write bindings to file
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("rdma_bindings.rs"))
        .expect("Couldn't write bindings");

    println!("cargo:warning=RDMA bindings generated successfully");
}

trait BindgenBuilderExt {
    fn header_wrapper(self) -> Self;
}

impl BindgenBuilderExt for bindgen::Builder {
    fn header_wrapper(self) -> Self {
        self.header_contents(
            "wrapper.h",
            r#"
#include <infiniband/verbs.h>
            "#,
        )
    }
}

mod bindings;

// We need to force DPDK to get included in linkage.
#[allow(unused)]
use dpdk_rs;

pub use bindings::*;

pub fn load_pcie_driver() {
    if std::env::var("DONT_SET_THIS").is_ok() {
        unsafe { spdk_nvme_pcie_set_hotplug_filter(std::mem::zeroed()) };
    }
}

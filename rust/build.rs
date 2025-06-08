fn main() {
    // println!("cargo::rustc-env=LD_LIBRARY_PATH=/nix/store/7ywk1r5az73cyk8c79g0r7vszl1bxc3h-ddcutil-2.2.0/lib");
    println!("cargo::rustc-link-search=/nix/store/7ywk1r5az73cyk8c79g0r7vszl1bxc3h-ddcutil-2.2.0/lib");
}
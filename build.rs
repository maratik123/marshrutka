fn main() {
    // workaround for https://github.com/longbridge/rust-i18n/issues/46
    println!("cargo:rerun-if-changed=locales");
}

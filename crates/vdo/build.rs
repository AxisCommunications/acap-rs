use std::env;

fn main() {
    println!("cargo::rustc-check-cfg=cfg(vdo_has_no_video)");
    if env::var("DEP_VDO_HAS_NO_VIDEO").as_deref() == Ok("true") {
        println!("cargo::rustc-cfg=vdo_has_no_video");
    }
}

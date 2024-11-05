use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Manifest {
    pub acap_package_conf: AcapPackageConf,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AcapPackageConf {
    pub setup: Setup,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Setup {
    pub app_name: String,
}

use super::*;

pub(crate) struct AdapterRegistry {
    taobao: erp_adapter_taobao::Adapter,
    jd: erp_adapter_jd::Adapter,
    pdd: erp_adapter_pdd::Adapter,
    douyin: erp_adapter_douyin::Adapter,
}

impl AdapterRegistry {
    pub(crate) fn new() -> Self {
        Self {
            taobao: erp_adapter_taobao::Adapter::new(),
            jd: erp_adapter_jd::Adapter::new(),
            pdd: erp_adapter_pdd::Adapter::new(),
            douyin: erp_adapter_douyin::Adapter::new(),
        }
    }

    pub(crate) fn platform_adapter(&self, platform: Platform) -> &dyn PlatformAdapter {
        match platform {
            Platform::Taobao => &self.taobao,
            Platform::Jd => &self.jd,
            Platform::Pdd => &self.pdd,
            Platform::Douyin => &self.douyin,
        }
    }
}

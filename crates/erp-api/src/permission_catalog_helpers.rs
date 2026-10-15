use super::*;

pub(crate) fn permission_catalog() -> Vec<PermissionDefinition> {
    PermissionCode::all()
        .iter()
        .copied()
        .map(|code| {
            let code_text = code.as_str();
            let (module, name, description) = describe_permission(code_text);

            PermissionDefinition {
                code: code_text.to_string(),
                name,
                module,
                description,
            }
        })
        .collect()
}

pub(crate) fn describe_permission(code: &str) -> (String, String, String) {
    let mut parts = code.split('.');
    let domain = parts.next().unwrap_or("SYS");
    let resource = parts.next().unwrap_or("RESOURCE");
    let action = parts.next().unwrap_or("READ");
    let resource_label = permission_resource_label(resource);
    let action_name = permission_action_name(action);
    let action_description = permission_action_description(action);

    (
        permission_module_label(domain, resource).to_string(),
        format!("{resource_label}{action_name}"),
        format!("允许{action_description}{resource_label}相关能力"),
    )
}

pub(crate) fn permission_module_label(domain: &str, resource: &str) -> &'static str {
    match (domain, resource) {
        ("OMS", "ORDER") => "订单中心",
        ("OMS", "SHIPPING") => "履约中心",
        ("OMS", "AFTERSALES") => "售后中心",
        ("SYS", _) => "系统管理",
        ("WMS", _) => "仓储中心",
        ("MFG", _) => "生产制造",
        ("BASIC", _) => "基础资料",
        ("PROCUREMENT", _) => "采购中心",
        ("PRINT", _) => "打印中心",
        ("OPS", _) => "运营中心",
        ("MARKETING", _) => "营销中心",
        _ => "平台能力",
    }
}

pub(crate) fn permission_resource_label(resource: &str) -> &'static str {
    match resource {
        "PERMISSION" => "权限目录",
        "ROLE" => "角色",
        "USER" => "用户",
        "AUDIT" => "审计日志",
        "SETTINGS" => "系统配置",
        "DLQ" => "死信任务",
        "ORDER" => "订单",
        "SHIPPING" => "发货",
        "AFTERSALES" => "售后",
        "INVENTORY" => "库存",
        "PRODUCTION" => "生产制造",
        "SKU" => "SKU",
        "KIT" => "组合品",
        "CHANNEL" => "渠道",
        "COURIER" => "快递",
        "SUPPLIER" => "供应商",
        "CUSTOMER" => "客户",
        "SUGGESTION" => "采购建议",
        "RETURN" => "采购退货",
        "REQUEST" => "采购申请",
        "PLAN" => "采购计划",
        "SOURCING" => "寻源",
        "INSPECTION" => "质检",
        "RECONCILIATION" => "对账",
        "PERFORMANCE" => "绩效",
        "MAPPING" => "映射规则",
        "WAREHOUSE" => "仓库",
        "CARRIER" => "承运商",
        "DEVICE" => "打印设备",
        "TEMPLATE" => "打印模板",
        "JOB" => "打印任务",
        "OPERATION_LOG" => "操作日志",
        "REPORT" => "报表",
        "EXPORT" => "导出任务",
        "LOGISTICS_PRODUCT" => "物流产品",
        "LOGISTICS_PRICE" => "物流价格",
        "LOGISTICS_MATCH_RULE" => "物流匹配规则",
        "ORDER_TAG" => "订单标签",
        "SKU_TAG" => "SKU标签",
        "SHOP_PRODUCT" => "店铺商品",
        "PRICE" => "价格策略",
        "PROMOTION" => "促销活动",
        "GIFT" => "赠品策略",
        "COUPON" => "优惠券",
        "COST" => "营销费用",
        "POSTMORTEM" => "复盘",
        _ => "权限项",
    }
}

pub(crate) fn permission_action_name(action: &str) -> &'static str {
    match action {
        "READ" => "查看",
        "WRITE" => "维护",
        "RAW" => "原始数据",
        "REVIEW" => "审核",
        "REPLAY" => "重放",
        _ => "操作",
    }
}

pub(crate) fn permission_action_description(action: &str) -> &'static str {
    match action {
        "READ" => "查看",
        "WRITE" => "维护",
        "RAW" => "查看",
        "REVIEW" => "审核",
        "REPLAY" => "重放",
        _ => "执行",
    }
}

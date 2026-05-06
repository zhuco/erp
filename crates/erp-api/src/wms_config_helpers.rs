use super::*;

pub(crate) const WMS_WORKFLOW_CONFIG_KEY: &str = "wms.workflow.config";
pub(crate) const WMS_LABOR_TASKS_KEY: &str = "wms.execution.labor.tasks";
pub(crate) const WMS_SLOTTING_SUGGESTIONS_KEY: &str = "wms.execution.slotting.suggestions";
pub(crate) const WMS_REPLENISHMENT_TASKS_KEY: &str = "wms.execution.replenishment.tasks";
pub(crate) const WMS_DOCK_APPOINTMENTS_KEY: &str = "wms.execution.dock.appointments";
pub(crate) const SYSTEM_WORKFLOW_CATALOG_KEY: &str = "system.workflow.catalog";
pub(crate) const SYSTEM_CUSTOM_FIELDS_CATALOG_KEY: &str = "system.custom-fields.catalog";
pub(crate) const SYSTEM_INTEGRATIONS_CATALOG_KEY: &str = "system.integrations.catalog";
pub(crate) const SYSTEM_AUDIT_REPORTS_CATALOG_KEY: &str = "system.audit-reports.catalog";

pub(crate) fn default_warehouse_workflow_config() -> WarehouseWorkflowConfigPayload {
    WarehouseWorkflowConfigPayload {
        enable_wave: false,
        require_inspection: false,
        require_weighing: false,
        require_pack: true,
    }
}

pub(crate) fn parse_workflow_config_value(value: &Value) -> Option<WarehouseWorkflowConfigPayload> {
    if let Ok(config) = serde_json::from_value::<WarehouseWorkflowConfigPayload>(value.clone()) {
        return Some(config);
    }
    if let Some(nested) = value.get("value") {
        if let Ok(config) = serde_json::from_value::<WarehouseWorkflowConfigPayload>(nested.clone())
        {
            return Some(config);
        }
    }
    None
}

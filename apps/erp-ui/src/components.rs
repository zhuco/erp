//! ERP-internal shared UI services. Reuse these instead of copying Lab modules.
//! These host adapters intentionally live outside the transport-free grid crates.
pub use crate::export::{
    Delivery, Format, SheetImage, SheetOptions, WorkbookSheet, encode, encode_workbook,
    encode_workbook_with_options,
};
pub use crate::named_views::{NamedViewsUi, storage_key};
pub use crate::selection_summary::SelectionSummary;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn audit_shared_export_preserves_signed_numbers_and_protects_text() {
        use erp_grid_core::{Column, ColumnId, ValueKind};
        let mut columns = vec![
            Column::new(ColumnId(1), "Quantity", 100.0),
            Column::new(ColumnId(2), "Money", 100.0),
            Column::new(ColumnId(3), "Text", 100.0),
        ];
        columns[0].value_kind = ValueKind::Integer;
        columns[1].value_kind = ValueKind::Money;
        let bytes = encode(
            &columns,
            &[vec!["-2".into(), "-1.25".into(), "=SUM(A1)".into()]],
            Format::Csv,
        )
        .unwrap();
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains("\"-2\",\"-1.25\",\"'=SUM(A1)\""));
        let _: SelectionSummary = Default::default();
        let views = NamedViewsUi::new(erp_grid_core::NamedViewScope::new("actor", "page", "grid"));
        assert!(views.is_ok());
    }
}

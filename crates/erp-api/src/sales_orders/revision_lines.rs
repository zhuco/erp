use super::*;
use sha2::{Digest, Sha256};

pub(super) fn prepare(
    req: &SalesOrderCreateRequest,
) -> Result<
    (
        Vec<erp_domain::trade::SalesLine>,
        Option<uuid::Uuid>,
        String,
    ),
    &'static str,
> {
    if req.items.is_empty() || req.items.len() > 1000 {
        return Err("sales revision requires 1..1000 lines");
    }
    if req.items.len() > 1 && req.tax_cent.is_some_and(|tax| tax != 0) {
        return Err("multi-line tax must be specified on each line");
    }
    let order = match req.so_id.as_deref() {
        Some(id) => {
            Some(uuid::Uuid::parse_str(id.trim()).map_err(|_| "invalid sales order identity")?)
        }
        None => None,
    };
    let mut seen = std::collections::BTreeSet::new();
    let mut lines = Vec::with_capacity(req.items.len());
    for (index, item) in req.items.iter().enumerate() {
        let line_no = item
            .line_no
            .unwrap_or(i32::try_from(index + 1).map_err(|_| "line number overflow")?);
        if line_no <= 0 || !seen.insert(line_no) {
            return Err("sales line identities must be positive and unique");
        }
        lines.push(erp_domain::trade::SalesLine {
            line_no,
            item_code: item.sku_code.as_deref().unwrap_or("").trim().into(),
            qty_micros: item.qty.revision_micros()?,
            price_cent: item.price_cent,
            discount_cent: item.discount_cent.unwrap_or(0),
            tax_cent: item.tax_cent.or(req.tax_cent).unwrap_or(0),
            due_date: item
                .due_date
                .as_deref()
                .or(req.due_date.as_deref())
                .unwrap_or("")
                .trim()
                .into(),
            address: item
                .address
                .as_deref()
                .or(req.address.as_deref())
                .or(req.receiver_address.as_deref())
                .unwrap_or("")
                .trim()
                .into(),
            source: item
                .source_kind
                .as_deref()
                .or(req.source_kind.as_deref())
                .unwrap_or("RESALE")
                .trim()
                .into(),
        });
    }
    // Include every effective line and version field; the command ID is not its fingerprint.
    let fields = lines
        .iter()
        .map(|line| {
            serde_json::json!([
                line.line_no,
                line.item_code,
                line.qty_micros,
                line.price_cent,
                line.discount_cent,
                line.tax_cent,
                line.due_date,
                line.address,
                line.source
            ])
        })
        .collect::<Vec<_>>();
    let payload = serde_json::to_vec(&serde_json::json!([
        1,
        order,
        req.customer_code.as_deref().unwrap_or("").trim(),
        req.expected_version,
        req.revision_action.as_deref().unwrap_or("SUBMIT").trim(),
        fields
    ]))
    .map_err(|_| "sales fingerprint unavailable")?;
    Ok((lines, order, format!("{:x}", Sha256::digest(payload))))
}

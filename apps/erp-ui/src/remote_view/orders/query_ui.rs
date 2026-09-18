use super::*;

pub(super) const SHOP: ColumnId = ColumnId(20);
pub(super) const RECEIVER: ColumnId = ColumnId(21);
pub(super) const PHONE: ColumnId = ColumnId(22);
pub(super) const CUSTOMER: ColumnId = ColumnId(23);
pub(super) const CREATED: ColumnId = ColumnId(24);
pub(super) const UPDATED: ColumnId = ColumnId(25);
pub(super) const REVIEW: ColumnId = ColumnId(26);
pub(super) const FULFILLMENT: ColumnId = ColumnId(27);
pub(super) const EXCEPTION: ColumnId = ColumnId(28);
pub(super) const PAYMENT: ColumnId = ColumnId(29);
pub(super) const TRACKING: ColumnId = ColumnId(30);
pub(super) const MAIN_STATUS: ColumnId = ColumnId(31);
pub(super) const FLAGS: ColumnId = ColumnId(32);

pub(super) fn platform_label(value: &str) -> &str {
    match value {
        "taobao" => "淘宝",
        "jd" => "京东",
        "pdd" => "拼多多",
        "douyin" => "抖音",
        _ => value,
    }
}

impl Orders {
    pub(super) fn query_toolbar(&mut self, ui: &mut egui::Ui) {
        let mut changed = false;
        ui.add_enabled_ui(!self.dirty(), |ui| {
            egui::ComboBox::from_id_salt("order-period")
                .width(85.0)
                .selected_text(if self.period_days == 0 {
                    "全部时间".into()
                } else {
                    format!("近 {} 天", self.period_days)
                })
                .show_ui(ui, |ui| {
                    for (days, label) in [
                        (0, "全部时间"),
                        (7, "近 7 天"),
                        (30, "近 30 天"),
                        (90, "近 90 天"),
                    ] {
                        changed |= ui
                            .selectable_value(&mut self.period_days, days, label)
                            .changed();
                    }
                });
            changed |= ui
                .add(
                    egui::TextEdit::singleline(&mut self.keyword)
                        .desired_width(220.0)
                        .hint_text("系统单号 / 平台单号 / 收件人"),
                )
                .changed();
            if ui.button("重置").clicked() {
                self.keyword.clear();
                self.period_days = 0;
                self.order_state.quick_search.clear();
                changed = true;
            }
        });
        if changed {
            self.invalidate_filter(ui.ctx());
        }
        let ready = self.list_initialized
            && self.list_task.is_none()
            && self.search_deadline.is_none()
            && !self.orders.is_empty();
        for (label, format) in [
            ("导出当页", crate::components::Format::Xlsx),
            ("打印当页", crate::components::Format::Html),
        ] {
            if ui
                .add_enabled(ready && self.delivery.is_none(), egui::Button::new(label))
                .clicked()
            {
                let columns: Vec<_> = self
                    .order_columns
                    .iter()
                    .filter(|c| c.visible)
                    .cloned()
                    .collect();
                let rows = self
                    .orders
                    .iter()
                    .map(|row| columns.iter().map(|c| order_text(row, c.id)).collect())
                    .collect::<Vec<_>>();
                match crate::components::encode(&columns, &rows, format) {
                    Ok(bytes) => {
                        self.delivery = Some(crate::components::Delivery::start(bytes, format))
                    }
                    Err(error) => self.notice = Some(error),
                }
            }
        }
    }

    pub(super) fn filter_tree(&mut self, ui: &mut egui::Ui) {
        let mut changed = false;
        ui.add_enabled_ui(!self.dirty(), |ui| {
            egui::CollapsingHeader::new("订单范围")
                .default_open(true)
                .show(ui, |ui| {
                    if ui
                        .selectable_label(
                            self.order_columns.iter().all(|c| {
                                self.order_state
                                    .quick_search
                                    .get(&c.id)
                                    .is_none_or(|d| matches!(d.compile(&c.filter), Ok(None)))
                            }),
                            "全部订单",
                        )
                        .clicked()
                    {
                        self.order_state.quick_search.clear();
                        changed = true;
                    }
                });
            for (id, title) in [(PLATFORM, "销售渠道"), (STATUS, "订单状态")] {
                if let Some(column) = self.order_columns.iter().find(|column| column.id == id)
                    && let FilterKind::Enum(values) = &column.filter
                {
                    egui::CollapsingHeader::new(title)
                        .default_open(true)
                        .show(ui, |ui| {
                            let mut selected = match self.order_state.quick_search.get(&id) {
                                Some(FilterDraft::Enum(values)) => values.clone(),
                                _ => Default::default(),
                            };
                            for value in values {
                                let mut checked = selected.contains(value);
                                let label = if id == PLATFORM {
                                    platform_label(value)
                                } else {
                                    status_label(value)
                                };
                                if ui.checkbox(&mut checked, label).changed() {
                                    if checked {
                                        selected.insert(value.clone());
                                    } else {
                                        selected.remove(value);
                                    }
                                    changed = true;
                                }
                            }
                            self.order_state
                                .quick_search
                                .insert(id, FilterDraft::Enum(selected));
                        });
                }
            }
        });
        if changed {
            self.invalidate_filter(ui.ctx());
        }
    }
}

pub(super) fn extra_query(
    state: &GridState,
    keyword: &str,
    days: u32,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<Vec<(String, String)>, &'static str> {
    let mut query = vec![("keyword".into(), keyword.trim().to_owned())];
    for (id, name) in [
        (ORDER_ID, "platform_order_id"),
        (RECEIVER, "receiver_name"),
        (PHONE, "receiver_phone"),
    ] {
        if let Some(draft) = state.quick_search.get(&id)
            && let Some(erp_grid_core::Filter::Text(_)) = draft.compile(&FilterKind::Text)?
            && let FilterDraft::Text(value) = draft
        {
            query.push((name.into(), value.trim().to_owned()));
        }
    }
    for (id, from, to) in [
        (CREATED, "created_from", "created_to"),
        (UPDATED, "updated_from", "updated_to"),
    ] {
        if let Some(draft) = state.quick_search.get(&id)
            && let FilterDraft::DateTime { start, end } = draft
        {
            draft.compile(&FilterKind::DateTime)?;
            for (key, bound) in [(from, start), (to, end)] {
                if let Some(parts) = bound {
                    query.push((key.into(), format!("{}Z", parts.label().replace(' ', "T"))));
                }
            }
        }
    }
    if days > 0
        && !query
            .iter()
            .any(|(key, _)| key == "created_from" || key == "created_to")
    {
        let start = now
            .checked_sub_signed(chrono::Duration::days(i64::from(days)))
            .ok_or("时间范围无效")?;
        query.push(("created_from".into(), start.to_rfc3339()));
        query.push(("created_to".into(), now.to_rfc3339()));
    }
    Ok(query)
}

pub(super) fn status_label(value: &str) -> &str {
    match value {
        "NEW" => "新建",
        "REVIEW_PENDING" => "待审核",
        "PENDING_SHIPMENT" => "待发货",
        "ALLOCATED" => "已分配",
        "PICKING" => "拣货中",
        "PACKED" => "已打包",
        "SHIPPED" => "已发货",
        "DONE" => "已完成",
        "CANCELLED" => "已取消",
        _ => value,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn query_keeps_global_keyword_and_column_predicates_distinct() {
        let mut state = GridState::default();
        state
            .quick_search
            .insert(ORDER_ID, FilterDraft::Text("QA-001".into()));
        state
            .quick_search
            .insert(RECEIVER, FilterDraft::Text("张".into()));
        let now = chrono::DateTime::from_timestamp(1_800_000_000, 0).expect("fixture timestamp");
        let query = extra_query(&state, "旗舰店", 7, now).expect("valid query");
        assert!(query.contains(&("keyword".into(), "旗舰店".into())));
        assert!(query.contains(&("platform_order_id".into(), "QA-001".into())));
        assert!(query.contains(&("receiver_name".into(), "张".into())));
        let start = query
            .iter()
            .find(|(key, _)| key == "created_from")
            .expect("start");
        assert_eq!(start.1, (now - chrono::Duration::days(7)).to_rfc3339());
        assert!(
            !extra_query(&state, "", 0, now)
                .expect("all time")
                .iter()
                .any(|(key, _)| key == "created_from")
        );
    }
}

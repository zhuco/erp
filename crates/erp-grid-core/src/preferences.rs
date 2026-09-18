use crate::{Column, ColumnId, Filter, FilterDraft, SortRule};
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ColumnPreference {
    pub id: ColumnId,
    pub width: f32,
    pub visible: bool,
    pub pinned: bool,
    #[serde(default)]
    pub pinned_right: bool,
}
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ViewPreferences {
    pub version: u32,
    pub scope: String,
    pub columns: Vec<ColumnPreference>,
    pub filters: Vec<(ColumnId, FilterDraft)>,
    pub sorts: Vec<SortRule>,
    pub page_size: usize,
    pub quick_search_open: bool,
    #[serde(default)]
    pub search: Option<String>,
    #[serde(default)]
    pub expression: Option<crate::paged_query::FilterExpression>,
}
impl ViewPreferences {
    pub fn apply_columns(
        &self,
        scope: &str,
        columns: &mut Vec<Column>,
    ) -> Result<(), &'static str> {
        if self.version != 1
            || self.scope != scope
            || self.columns.len() > 1000
            || self.filters.len() > 1000
        {
            return Err("Unsupported or mismatched view settings");
        }
        let mut ordered = Vec::new();
        let mut remaining = columns.clone();
        for preference in &self.columns {
            if let Some(index) = remaining
                .iter()
                .position(|column| column.id == preference.id)
            {
                let mut column = remaining.remove(index);
                column.resize(preference.width);
                column.visible = preference.visible;
                column.pinned = preference.pinned;
                column.pinned_right = preference.pinned_right && !preference.pinned;
                ordered.push(column);
            }
        }
        ordered.extend(remaining);
        if !ordered.iter().any(|c| c.visible)
            && let Some(column) = ordered.first_mut()
        {
            column.visible = true;
        }
        ordered.sort_by_key(Column::pin_rank);
        *columns = ordered;
        Ok(())
    }
}

/// Versioned identity for saved views. A view can only be read in the exact
/// account, page and widget instance in which it was created.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NamedViewScope {
    pub account: String,
    pub page: String,
    pub instance: String,
}

impl NamedViewScope {
    pub fn new(
        account: impl Into<String>,
        page: impl Into<String>,
        instance: impl Into<String>,
    ) -> Self {
        Self {
            account: account.into(),
            page: page.into(),
            instance: instance.into(),
        }
    }

    pub fn preference_scope(&self) -> String {
        format!(
            "named-v1:a{}:{}:p{}:{}:i{}:{}",
            self.account.len(),
            self.account,
            self.page.len(),
            self.page,
            self.instance.len(),
            self.instance
        )
    }

    fn validate(&self) -> Result<(), String> {
        for (label, value) in [
            ("account", &self.account),
            ("page", &self.page),
            ("instance", &self.instance),
        ] {
            if value.is_empty() || value.len() > 256 || value.contains('\0') {
                return Err(format!("Invalid named-view {label} scope"));
            }
        }
        Ok(())
    }
}

pub const NAMED_VIEW_VERSION: u32 = 1;
pub const MAX_NAMED_VIEWS: usize = 64;
pub const MAX_NAMED_VIEW_NAME_BYTES: usize = 128;
pub const MAX_NAMED_VIEW_TEXT_BYTES: usize = 64 * 1024;
pub const MAX_NAMED_VIEW_TOTAL_TEXT_BYTES: usize = 1024 * 1024;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NamedView {
    pub name: String,
    pub preferences: ViewPreferences,
}

/// Bounded, account/page/instance-scoped collection of reusable grid views.
/// Mutations validate the complete candidate before replacing the collection.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NamedViewStore {
    pub version: u32,
    pub scope: NamedViewScope,
    pub views: Vec<NamedView>,
}

impl NamedViewStore {
    pub fn new(scope: NamedViewScope) -> Result<Self, String> {
        scope.validate()?;
        Ok(Self {
            version: NAMED_VIEW_VERSION,
            scope,
            views: Vec::new(),
        })
    }

    pub fn list(&self) -> &[NamedView] {
        &self.views
    }

    pub fn get(&self, name: &str) -> Option<&ViewPreferences> {
        self.views
            .iter()
            .find(|view| view.name == name)
            .map(|view| &view.preferences)
    }

    pub fn save(
        &mut self,
        name: impl Into<String>,
        preferences: ViewPreferences,
    ) -> Result<(), String> {
        let name = name.into();
        validate_named_view_name(&name)?;
        validate_preferences(&self.scope, &preferences)?;
        let mut candidate = self.views.clone();
        if let Some(existing) = candidate.iter_mut().find(|view| view.name == name) {
            existing.preferences = preferences;
        } else {
            if candidate.len() >= MAX_NAMED_VIEWS {
                return Err("Named-view limit reached".into());
            }
            candidate.push(NamedView { name, preferences });
        }
        let candidate_store = Self {
            version: self.version,
            scope: self.scope.clone(),
            views: candidate.clone(),
        };
        candidate_store.validate(&self.scope)?;
        self.views = candidate;
        Ok(())
    }

    pub fn rename(&mut self, old: &str, new: impl Into<String>) -> Result<(), String> {
        let new = new.into();
        validate_named_view_name(&new)?;
        let mut candidate = self.views.clone();
        if !candidate.iter().any(|view| view.name == old) {
            return Err("Named view not found".into());
        }
        if old != new && candidate.iter().any(|view| view.name == new) {
            return Err("Named view already exists".into());
        }
        let Some(view) = candidate.iter_mut().find(|view| view.name == old) else {
            return Err("Named view not found".into());
        };
        view.name = new;
        let candidate_store = Self {
            version: self.version,
            scope: self.scope.clone(),
            views: candidate.clone(),
        };
        candidate_store.validate(&self.scope)?;
        self.views = candidate;
        Ok(())
    }

    pub fn delete(&mut self, name: &str) -> Result<(), String> {
        let mut candidate = self.views.clone();
        let before = candidate.len();
        candidate.retain(|view| view.name != name);
        if candidate.len() == before {
            return Err("Named view not found".into());
        }
        let candidate_store = Self {
            version: self.version,
            scope: self.scope.clone(),
            views: candidate.clone(),
        };
        candidate_store.validate(&self.scope)?;
        self.views = candidate;
        Ok(())
    }

    pub fn validate(&self, expected: &NamedViewScope) -> Result<(), String> {
        if self.version != NAMED_VIEW_VERSION || &self.scope != expected {
            return Err("Named-view version or scope mismatch".into());
        }
        self.scope.validate()?;
        if self.views.len() > MAX_NAMED_VIEWS {
            return Err("Too many named views".into());
        }
        let mut total_text = 0usize;
        for (index, view) in self.views.iter().enumerate() {
            validate_named_view_name(&view.name)?;
            if self.views[..index]
                .iter()
                .any(|previous| previous.name == view.name)
            {
                return Err("Duplicate named view".into());
            }
            validate_preferences(&self.scope, &view.preferences)?;
            if preference_text_bytes(&view.preferences) > MAX_NAMED_VIEW_TEXT_BYTES {
                return Err("Named-view text exceeds per-view limit".into());
            }
            total_text = total_text
                .checked_add(view.name.len())
                .and_then(|total| total.checked_add(preference_text_bytes(&view.preferences)))
                .ok_or_else(|| "Named-view text exceeds limit".to_owned())?;
            if total_text > MAX_NAMED_VIEW_TOTAL_TEXT_BYTES {
                return Err("Named-view text exceeds limit".into());
            }
        }
        Ok(())
    }
}

fn preference_text_bytes(preferences: &ViewPreferences) -> usize {
    preferences.scope.len()
        + preferences.search.as_ref().map_or(0, String::len)
        + preferences
            .expression
            .as_ref()
            .map_or(0, expression_text_bytes)
        + preferences
            .filters
            .iter()
            .map(|(_, filter)| match filter {
                FilterDraft::Text(value) => value.len(),
                FilterDraft::Money { min, max } => min.len() + max.len(),
                FilterDraft::DateTime { .. } => 0,
                FilterDraft::Enum(values) => values.iter().map(String::len).sum(),
            })
            .sum::<usize>()
}

fn expression_text_bytes(expression: &crate::paged_query::FilterExpression) -> usize {
    use crate::paged_query::FilterExpression;

    match expression {
        FilterExpression::Condition { filter, .. } => match filter {
            Filter::Text(value) => value.len(),
            Filter::Enum(values) => values.iter().map(String::len).sum(),
            Filter::Money { .. } | Filter::DateTime { .. } => 0,
        },
        FilterExpression::And(items) | FilterExpression::Or(items) => {
            items.iter().map(expression_text_bytes).sum()
        }
    }
}

fn validate_named_view_name(name: &str) -> Result<(), String> {
    if name.trim().is_empty() || name.len() > MAX_NAMED_VIEW_NAME_BYTES || name.contains('\0') {
        Err("Invalid named-view name".into())
    } else {
        Ok(())
    }
}

fn validate_preferences(
    scope: &NamedViewScope,
    preferences: &ViewPreferences,
) -> Result<(), String> {
    if preferences.version != 1
        || preferences.scope != scope.preference_scope()
        || preferences.scope.is_empty()
        || preferences.scope.len() > 1024
        || preferences.scope.contains('\0')
        || preferences.columns.len() > 1000
        || preferences.filters.len() > 1000
        || preferences.sorts.len() > 1000
        || preferences.page_size == 0
        || preferences
            .search
            .as_ref()
            .is_some_and(|search| search.len() > 4096 || search.contains('\0'))
    {
        return Err("Invalid named-view preferences".into());
    }
    if let Some(expression) = &preferences.expression {
        expression
            .validate()
            .map_err(|error| format!("Invalid named-view expression: {error}"))?;
    }
    for (_, filter) in &preferences.filters {
        let valid = match filter {
            FilterDraft::Text(value) => value.len() <= MAX_NAMED_VIEW_TEXT_BYTES,
            FilterDraft::Money { min, max } => {
                min.len() <= MAX_NAMED_VIEW_TEXT_BYTES && max.len() <= MAX_NAMED_VIEW_TEXT_BYTES
            }
            FilterDraft::DateTime { .. } => true,
            FilterDraft::Enum(values) => {
                values.len() <= 1000
                    && values
                        .iter()
                        .all(|value| value.len() <= MAX_NAMED_VIEW_TEXT_BYTES)
            }
        };
        if !valid {
            return Err("Named-view filter data exceeds limit".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod named_view_tests {
    use super::*;

    #[test]
    fn right_pins_round_trip_and_legacy_left_pin_keeps_its_meaning() {
        let legacy: ColumnPreference =
            serde_json::from_str(r#"{"id":1,"width":120,"visible":true,"pinned":true}"#).unwrap();
        assert!(legacy.pinned && !legacy.pinned_right);
        let scope = NamedViewScope::new("a", "p", "i");
        let mut view = prefs(&scope);
        view.columns = vec![
            ColumnPreference {
                id: ColumnId(2),
                width: 120.0,
                visible: true,
                pinned: false,
                pinned_right: true,
            },
            legacy,
        ];
        let restored: ViewPreferences =
            serde_json::from_str(&serde_json::to_string(&view).unwrap()).unwrap();
        let mut columns = vec![
            Column::new(ColumnId(1), "A", 120.0),
            Column::new(ColumnId(2), "B", 120.0),
            Column::new(ColumnId(3), "C", 120.0),
        ];
        restored
            .apply_columns(&scope.preference_scope(), &mut columns)
            .unwrap();
        assert_eq!(
            columns
                .iter()
                .map(|c| (c.id, c.pin_rank()))
                .collect::<Vec<_>>(),
            vec![(ColumnId(1), 0), (ColumnId(3), 1), (ColumnId(2), 2)]
        );
    }

    fn prefs(scope: &NamedViewScope) -> ViewPreferences {
        ViewPreferences {
            version: 1,
            scope: scope.preference_scope(),
            columns: vec![],
            filters: vec![],
            sorts: vec![],
            page_size: 20,
            quick_search_open: true,
            search: None,
            expression: None,
        }
    }

    #[test]
    fn save_overwrite_rename_delete_and_scope_isolation() {
        let scope = NamedViewScope::new("acct", "products", "main");
        let mut store = NamedViewStore::new(scope.clone()).unwrap();
        store.save("Default", prefs(&scope)).unwrap();
        store
            .save(
                "Default",
                ViewPreferences {
                    page_size: 50,
                    ..prefs(&scope)
                },
            )
            .unwrap();
        assert_eq!(store.list().len(), 1);
        assert_eq!(store.get("Default").unwrap().page_size, 50);
        store.rename("Default", "Wide").unwrap();
        assert!(store.delete("Wide").is_ok());
        let other = NamedViewScope::new("acct", "orders", "main");
        assert!(store.validate(&other).is_err());
    }

    #[test]
    fn corrupt_or_future_input_is_rejected_without_mutating_store() {
        let scope = NamedViewScope::new("acct", "products", "main");
        let mut store = NamedViewStore::new(scope.clone()).unwrap();
        store.save("Keep", prefs(&scope)).unwrap();
        let before = store.clone();
        assert!(store.save("", prefs(&scope)).is_err());
        assert_eq!(store, before);
        let mut future = before.clone();
        future.version = 2;
        assert!(future.validate(&scope).is_err());
        assert!(NamedViewStore::new(NamedViewScope::new("", "products", "main")).is_err());
    }

    #[test]
    fn legacy_json_defaults_query_fields() {
        let scope = NamedViewScope::new("acct", "products", "main");
        let legacy = format!(
            r#"{{"version":1,"scope":"{}","columns":[],"filters":[],"sorts":[],"page_size":20,"quick_search_open":true}}"#,
            scope.preference_scope()
        );
        let decoded: ViewPreferences = serde_json::from_str(&legacy).unwrap();
        assert_eq!(decoded.search, None);
        assert_eq!(decoded.expression, None);
    }

    #[test]
    fn full_query_round_trips_through_named_view_json() {
        use crate::paged_query::FilterExpression;

        let scope = NamedViewScope::new("acct", "products", "main");
        let mut preferences = prefs(&scope);
        preferences.search = Some("winter coat".into());
        preferences.expression = Some(FilterExpression::Or(vec![
            FilterExpression::Condition {
                column: ColumnId(1),
                filter: Filter::Text("ready".into()),
            },
            FilterExpression::Condition {
                column: ColumnId(2),
                filter: Filter::Enum(["urgent".into(), "held".into()].into()),
            },
        ]));
        let mut store = NamedViewStore::new(scope.clone()).unwrap();
        store.save("Operational", preferences.clone()).unwrap();
        let json = serde_json::to_string(&store).unwrap();
        let decoded: NamedViewStore = serde_json::from_str(&json).unwrap();
        decoded.validate(&scope).unwrap();
        assert_eq!(decoded.get("Operational"), Some(&preferences));
    }

    #[test]
    fn invalid_query_save_is_atomic() {
        use crate::paged_query::FilterExpression;

        let scope = NamedViewScope::new("acct", "products", "main");
        let mut store = NamedViewStore::new(scope.clone()).unwrap();
        store.save("Keep", prefs(&scope)).unwrap();
        let before = store.clone();

        let mut invalid_search = prefs(&scope);
        invalid_search.search = Some("x".repeat(4097));
        assert!(store.save("Keep", invalid_search).is_err());
        assert_eq!(store, before);

        let mut nul_search = prefs(&scope);
        nul_search.search = Some("bad\0search".into());
        assert!(store.save("Keep", nul_search).is_err());
        assert_eq!(store, before);

        let mut invalid_expression = prefs(&scope);
        invalid_expression.expression = Some(FilterExpression::And(vec![]));
        assert!(store.save("Keep", invalid_expression).is_err());
        assert_eq!(store, before);
    }
}

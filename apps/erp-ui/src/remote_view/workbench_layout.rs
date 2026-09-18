use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct LayoutPreferences {
    pub left_width: f32,
    pub right_width: f32,
    pub bottom_height: f32,
    pub left_open: bool,
    pub right_open: bool,
    pub bottom_open: bool,
}
impl Default for LayoutPreferences {
    fn default() -> Self {
        Self {
            left_width: crate::erp_theme::LEFT_WIDTH,
            right_width: crate::erp_theme::RIGHT_WIDTH,
            bottom_height: crate::erp_theme::DETAILS_HEIGHT,
            left_open: true,
            right_open: false,
            bottom_open: true,
        }
    }
}
impl LayoutPreferences {
    fn valid(&self) -> bool {
        self.left_width.is_finite()
            && (160.0..=420.0).contains(&self.left_width)
            && self.right_width.is_finite()
            && (180.0..=1000.0).contains(&self.right_width)
            && self.bottom_height.is_finite()
            && (100.0..=4000.0).contains(&self.bottom_height)
    }
}
#[derive(Clone, Debug, Default)]
pub(crate) struct WorkbenchLayout {
    pub preferences: LayoutPreferences,
    scope: Option<String>,
    dirty: bool,
}
impl WorkbenchLayout {
    pub fn sync(
        &mut self,
        storage: &mut dyn eframe::Storage,
        scope: Option<&str>,
        page: &str,
    ) -> bool {
        let key = scope.map(|scope| format!("erp.workbench.layout.v1.{page}.{scope}"));
        if self.scope == key {
            return false;
        }
        self.scope = key;
        self.preferences = self
            .scope
            .as_ref()
            .and_then(|key| storage.get_string(key))
            .filter(|value| value.len() <= 4096)
            .and_then(|value| serde_json::from_str::<LayoutPreferences>(&value).ok())
            .filter(LayoutPreferences::valid)
            .unwrap_or_default();
        self.dirty = false;
        true
    }
    pub fn scope_key(&self) -> &str {
        self.scope.as_deref().unwrap_or("anonymous")
    }
    pub fn persist(&mut self, storage: &mut dyn eframe::Storage, _page: &str) {
        if !self.dirty {
            return;
        }
        let Some(key) = &self.scope else {
            return;
        };
        if let Ok(value) = serde_json::to_string(&self.preferences) {
            storage.set_string(key, value);
            self.dirty = false;
        }
    }
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }
    pub fn set_left_width(&mut self, width: f32) {
        let width = width.clamp(160.0, 420.0);
        if self.preferences.left_width != width {
            self.preferences.left_width = width;
            self.dirty = true;
        }
    }
    pub fn set_right_width(&mut self, width: f32) {
        let width = width.clamp(180.0, 1000.0);
        if self.preferences.right_width != width {
            self.preferences.right_width = width;
            self.dirty = true;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct Storage(std::collections::BTreeMap<String, String>);
    impl eframe::Storage for Storage {
        fn get_string(&self, key: &str) -> Option<String> {
            self.0.get(key).cloned()
        }
        fn set_string(&mut self, key: &str, value: String) {
            self.0.insert(key.into(), value);
        }
        fn remove_string(&mut self, key: &str) {
            self.0.remove(key);
        }
        fn flush(&mut self) {}
    }
    #[test]
    fn geometry_isolated_by_account_and_page_and_not_reset_each_frame() {
        let mut storage = Storage::default();
        let mut layout = WorkbenchLayout::default();
        layout.sync(&mut storage, Some("a"), "products");
        layout.set_left_width(300.0);
        layout.sync(&mut storage, Some("a"), "products");
        assert_eq!(layout.preferences.left_width, 300.0);
        layout.persist(&mut storage, "products");
        layout.sync(&mut storage, Some("a"), "orders");
        assert_eq!(layout.preferences.left_width, 240.0);
        layout.sync(&mut storage, Some("b"), "products");
        assert_eq!(layout.preferences.left_width, 240.0);
        layout.sync(&mut storage, Some("a"), "products");
        assert_eq!(layout.preferences.left_width, 300.0);
    }
}

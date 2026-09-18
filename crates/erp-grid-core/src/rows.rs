use std::collections::BTreeSet;

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(try_from = "String", into = "String")]
pub struct RowId(pub u128);

impl RowId {
    pub fn from_uuid(value: &str) -> Option<Self> {
        if value.len() != 36 || !value.is_ascii() {
            return None;
        }
        let mut hex = String::with_capacity(32);
        for (index, ch) in value.chars().enumerate() {
            if [8, 13, 18, 23].contains(&index) {
                if ch != '-' {
                    return None;
                }
            } else if ch.is_ascii_hexdigit() {
                hex.push(ch);
            } else {
                return None;
            }
        }
        u128::from_str_radix(&hex, 16).ok().map(Self)
    }
    pub fn to_uuid(self) -> String {
        let hex = format!("{:032x}", self.0);
        format!(
            "{}-{}-{}-{}-{}",
            &hex[..8],
            &hex[8..12],
            &hex[12..16],
            &hex[16..20],
            &hex[20..]
        )
    }
}
impl TryFrom<String> for RowId {
    type Error = &'static str;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_uuid(&value).ok_or("Invalid UUID row identity")
    }
}
impl From<RowId> for String {
    fn from(value: RowId) -> Self {
        value.to_uuid()
    }
}

/// Business targets are independent of the rectangular clipboard selection.
#[derive(Default, Debug, Clone)]
pub struct RowSelection {
    pub active: Option<RowId>,
    checked: BTreeSet<RowId>,
    overflowed: bool,
}

impl RowSelection {
    pub const MAX_CHECKED: usize = 10_000;
    pub fn take_overflowed(&mut self) -> bool {
        std::mem::take(&mut self.overflowed)
    }
    pub fn retain_checked(&mut self, keep: impl FnMut(&RowId) -> bool) {
        self.checked.retain(keep);
    }
    pub fn checked(&self) -> &BTreeSet<RowId> {
        &self.checked
    }
    pub fn toggle(&mut self, id: RowId) {
        if !self.checked.remove(&id) {
            self.set_checked(id, true);
        }
    }
    pub fn clear_checked(&mut self) {
        self.checked.clear();
        self.overflowed = false;
    }
    pub fn set_checked(&mut self, id: RowId, checked: bool) {
        if checked {
            if self.checked.len() >= Self::MAX_CHECKED && !self.checked.contains(&id) {
                self.overflowed = true;
                return;
            }
            self.checked.insert(id);
        } else {
            self.checked.remove(&id);
        }
    }
    pub fn clear(&mut self) {
        self.active = None;
        self.clear_checked();
    }
    pub fn target_count(&self) -> usize {
        if self.checked.is_empty() {
            usize::from(self.active.is_some())
        } else {
            self.checked.len()
        }
    }
    /// Freeze this list on action invocation; never recompute midway through execution.
    pub fn targets(&self) -> Vec<RowId> {
        if self.checked.is_empty() {
            self.active.into_iter().collect()
        } else {
            self.checked.iter().copied().collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn checked_limit_preserves_existing_ids_and_allows_unchecking() {
        let mut rows = RowSelection::default();
        for id in 0..RowSelection::MAX_CHECKED as u128 {
            rows.set_checked(RowId(id), true);
        }
        rows.toggle(RowId(10001));
        assert!(rows.take_overflowed());
        assert!(!rows.take_overflowed());
        assert_eq!(rows.checked().len(), 10000);
        assert!(rows.checked().contains(&RowId(0)));
        assert!(!rows.checked().contains(&RowId(10001)));
        rows.toggle(RowId(0));
        rows.toggle(RowId(10001));
        assert_eq!(rows.checked().len(), 10000);
        assert!(rows.checked().contains(&RowId(10001)));
        assert!(!rows.take_overflowed());
    }
    #[test]
    fn checked_rows_override_active_then_fall_back_when_last_is_unchecked() {
        let mut rows = RowSelection::default();
        assert!(rows.targets().is_empty());
        rows.active = Some(RowId(11));
        assert_eq!(rows.targets(), [RowId(11)]);
        assert!(rows.checked().is_empty());
        rows.toggle(RowId(22));
        rows.toggle(RowId(33));
        rows.active = Some(RowId(44));
        assert_eq!(rows.targets(), [RowId(22), RowId(33)]);
        rows.toggle(RowId(22));
        rows.toggle(RowId(33));
        assert_eq!(rows.targets(), [RowId(44)]);
        rows.clear();
        assert!(rows.targets().is_empty());
    }
}

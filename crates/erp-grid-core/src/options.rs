//! Bounded, cancellable remote option-directory lifecycle.
use crate::{ColumnId, RowId};
use std::collections::BTreeSet;

pub const MAX_OPTION_VALUES: usize = 100;
pub const MAX_OPTION_VALUE_BYTES: usize = 4096;
pub const MAX_OPTION_SEARCH_BYTES: usize = 4096;
pub const MAX_OPTION_PAGE_SIZE: u32 = 100;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OptionIdentity {
    pub tenant: RowId,
    pub query_scope: String,
    pub filter_scope: String,
    pub column: ColumnId,
    pub search: Option<String>,
    pub offset: u64,
}

impl OptionIdentity {
    pub fn validate(&self) -> Result<(), String> {
        for (label, value) in [("query", &self.query_scope), ("filter", &self.filter_scope)] {
            if value.is_empty() || value.len() > 256 || value.contains('\0') {
                return Err(format!("Invalid option {label} scope"));
            }
        }
        if self.search.as_ref().is_some_and(|value| {
            value.is_empty() || value.len() > MAX_OPTION_SEARCH_BYTES || value.contains('\0')
        }) {
            return Err("Invalid option search".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OptionRequest {
    pub generation: u64,
    pub identity: OptionIdentity,
    pub limit: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OptionPage {
    pub values: Vec<String>,
    pub total: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OptionAcceptance {
    Stored,
    Stale,
    Failed(String),
}

#[derive(Default)]
pub struct OptionDirectory {
    generation: u64,
    pending: Option<OptionRequest>,
    identity: Option<OptionIdentity>,
    pub values: Vec<String>,
    pub total: Option<u64>,
    pub loading: bool,
    pub error: Option<String>,
}

impl OptionDirectory {
    pub fn bind(&mut self, identity: OptionIdentity) -> Option<OptionRequest> {
        let cancelled = self.pending.take();
        self.generation = self.generation.wrapping_add(1);
        self.identity = Some(identity);
        self.values.clear();
        self.total = None;
        self.error = None;
        self.loading = false;
        cancelled
    }

    pub fn start(&mut self, limit: u32) -> Result<OptionRequest, String> {
        let identity = self.identity.clone().ok_or("Bind an option query first")?;
        identity.validate()?;
        if !(1..=MAX_OPTION_PAGE_SIZE).contains(&limit)
            || identity.offset > u64::MAX - u64::from(limit)
        {
            return Err("Invalid option page range".into());
        }
        if self.pending.is_some() {
            return Err("Option request already pending".into());
        }
        self.generation = self.generation.wrapping_add(1);
        let request = OptionRequest {
            generation: self.generation,
            identity,
            limit,
        };
        self.pending = Some(request.clone());
        self.loading = true;
        self.error = None;
        Ok(request)
    }

    pub fn cancel(&mut self) -> Option<OptionRequest> {
        self.loading = false;
        self.values.clear();
        self.total = None;
        self.pending.take()
    }

    pub fn accept(
        &mut self,
        request: &OptionRequest,
        response: Result<OptionPage, String>,
    ) -> OptionAcceptance {
        if self.pending.as_ref() != Some(request) || request.generation != self.generation {
            return OptionAcceptance::Stale;
        }
        self.pending = None;
        self.loading = false;
        match response.and_then(|page| validate_page(request, page)) {
            Ok(page) => {
                self.values = page.values;
                self.total = Some(page.total);
                self.error = None;
                OptionAcceptance::Stored
            }
            Err(error) => {
                let error: String = error.chars().take(1024).collect();
                self.error = Some(error.clone());
                OptionAcceptance::Failed(error)
            }
        }
    }

    pub fn retry(&mut self) -> Result<OptionRequest, String> {
        let limit = self
            .pending
            .as_ref()
            .map_or(MAX_OPTION_PAGE_SIZE, |request| request.limit);
        self.start(limit)
    }

    pub fn complete_dropdown(&self) -> bool {
        !self.loading
            && self.error.is_none()
            && self.total.is_some_and(|total| {
                total <= MAX_OPTION_VALUES as u64
                    && self.values.len() as u64 == total
                    && self
                        .identity
                        .as_ref()
                        .is_some_and(|identity| identity.offset == 0 && identity.search.is_none())
            })
    }
}

fn validate_page(request: &OptionRequest, page: OptionPage) -> Result<OptionPage, String> {
    if page.values.len() > request.limit as usize || page.total < page.values.len() as u64 {
        return Err("Option response exceeds requested page or has invalid total".into());
    }
    if !page.values.is_empty()
        && request.identity.offset > page.total.saturating_sub(page.values.len() as u64)
    {
        return Err("Option response does not match requested offset".into());
    }
    let mut unique = BTreeSet::new();
    for value in &page.values {
        if value.len() > MAX_OPTION_VALUE_BYTES || value.contains('\0') || !unique.insert(value) {
            return Err("Option response contains invalid or duplicate values".into());
        }
    }
    Ok(page)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn identity(offset: u64) -> OptionIdentity {
        OptionIdentity {
            tenant: RowId(1),
            query_scope: "products".into(),
            filter_scope: "brand".into(),
            column: ColumnId(2),
            search: None,
            offset,
        }
    }
    #[test]
    fn generation_cancellation_and_stale_response_are_rejected() {
        let mut directory = OptionDirectory::default();
        directory.bind(identity(0));
        let old = directory.start(10).unwrap();
        directory.bind(identity(10));
        assert_eq!(
            directory.accept(
                &old,
                Ok(OptionPage {
                    values: vec!["A".into()],
                    total: 1
                })
            ),
            OptionAcceptance::Stale
        );
        let fresh = directory.start(10).unwrap();
        assert_eq!(
            directory.accept(
                &fresh,
                Ok(OptionPage {
                    values: vec!["B".into()],
                    total: 11
                })
            ),
            OptionAcceptance::Stored
        );
    }
    #[test]
    fn duplicate_values_and_large_catalogue_do_not_become_dropdown() {
        let mut directory = OptionDirectory::default();
        directory.bind(identity(0));
        let request = directory.start(2).unwrap();
        assert!(matches!(
            directory.accept(
                &request,
                Ok(OptionPage {
                    values: vec!["A".into(), "A".into()],
                    total: 2
                })
            ),
            OptionAcceptance::Failed(_)
        ));
        let request = directory.start(2).unwrap();
        assert_eq!(
            directory.accept(
                &request,
                Ok(OptionPage {
                    values: vec!["A".into(), "B".into()],
                    total: 101
                })
            ),
            OptionAcceptance::Stored
        );
        assert!(!directory.complete_dropdown());
    }
}

use erp_grid_core::report::{GroupSummary, ReportResult, Summary};
use std::collections::BTreeMap;

// Input groups are disjoint leaf buckets. Prefix totals never feed back into
// this accumulation, so expanding a group cannot double-count its children.
pub(crate) fn rollup(result: &ReportResult) -> Result<Vec<GroupSummary>, String> {
    let mut groups: BTreeMap<Vec<String>, GroupSummary> = BTreeMap::new();
    for leaf in &result.groups {
        if leaf.key.is_empty() || leaf.key.len() > 8 {
            return Err("Invalid report hierarchy depth".into());
        }
        for depth in 1..=leaf.key.len() {
            let key = leaf.key[..depth].to_vec();
            if !groups.contains_key(&key) && groups.len() >= 10_000 {
                return Err("Report hierarchy exceeds 10,000 groups".into());
            }
            let group = groups.entry(key.clone()).or_insert_with(|| GroupSummary {
                key,
                summary: Summary::default(),
                pivot: BTreeMap::new(),
            });
            merge(&mut group.summary, &leaf.summary)?;
            for (pivot, summary) in &leaf.pivot {
                merge(group.pivot.entry(pivot.clone()).or_default(), summary)?;
            }
        }
    }
    Ok(groups.into_values().collect())
}

fn merge(target: &mut Summary, source: &Summary) -> Result<(), String> {
    target.count = target
        .count
        .checked_add(source.count)
        .ok_or("Group count overflow")?;
    target.sum = target
        .sum
        .checked_add(source.sum)
        .ok_or("Group sum overflow")?;
    target.min = target.min.into_iter().chain(source.min).min();
    target.max = target.max.into_iter().chain(source.max).max();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use erp_grid_core::report::ReportAccumulator;

    #[test]
    fn hierarchy_rollup_keeps_parent_totals_and_months_without_double_counting() {
        let mut report = ReportAccumulator::new(20, 12).unwrap();
        report
            .push(vec!["North".into(), "A".into()], "01".into(), 120)
            .unwrap();
        report
            .push(vec!["North".into(), "B".into()], "02".into(), -20)
            .unwrap();
        let report = report.finish();
        let groups = rollup(&report).unwrap();
        assert_eq!(groups.len(), 3);
        assert_eq!(groups[0].summary.count, 2);
        assert_eq!(groups[0].summary.sum, 100);
        assert_eq!(groups[0].summary.min, Some(-20));
        assert_eq!(groups[0].pivot["01"].sum, 120);
        assert_eq!(report.total.sum, 100);
    }
}

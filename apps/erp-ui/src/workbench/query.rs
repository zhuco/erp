//! Admission for actions against the last applied query.
//! Hosts retain query identity and cancel their own tasks as soon as input changes.

#[derive(Default)]
pub struct QueryAdmission {
    deadline: Option<f64>,
    composing: bool,
}

impl QueryAdmission {
    pub fn changed(&mut self, now: f64) {
        self.deadline = Some(now + 0.25);
    }

    pub fn immediate(&mut self, now: f64) {
        self.deadline = Some(now);
    }

    pub fn reset(&mut self, now: f64) {
        self.composing = false;
        self.immediate(now);
    }

    pub fn composition(&mut self, active: bool, now: f64) {
        if self.composing && !active && self.deadline.is_some() {
            self.changed(now);
        }
        self.composing = active;
    }

    pub fn ready(&self) -> bool {
        !self.composing && self.deadline.is_none()
    }

    pub fn pending(&self) -> bool {
        self.deadline.is_some()
    }

    pub fn due(&self, now: f64) -> bool {
        !self.composing && self.deadline.is_some_and(|deadline| now >= deadline)
    }

    pub fn applied(&mut self) {
        self.deadline = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_query_composition_blocks_expired_input_and_restarts_on_commit() {
        let mut admission = QueryAdmission::default();
        admission.composition(true, 1.0);
        admission.changed(1.0);
        assert!(!admission.ready());
        assert!(!admission.due(2.0));
        admission.composition(false, 2.0);
        assert!(!admission.due(2.249));
        assert!(admission.due(2.25));
        admission.applied();
        assert!(admission.ready());
    }

    #[test]
    fn module_query_unrelated_composition_does_not_create_a_query() {
        let mut admission = QueryAdmission::default();
        admission.composition(true, 1.0);
        assert!(!admission.ready());
        admission.composition(false, 2.0);
        assert!(admission.ready());
        assert!(!admission.due(2.0));
    }

    #[test]
    fn module_query_reset_replaces_old_delay_and_composition() {
        let mut admission = QueryAdmission::default();
        admission.composition(true, 1.0);
        admission.changed(1.0);
        admission.reset(1.01);
        assert!(admission.due(1.01));
        assert!(!admission.ready());
        admission.applied();
        assert!(admission.ready());
    }
}

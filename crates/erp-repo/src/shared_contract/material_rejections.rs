//! Only new commands rejected before COMMIT receive a definite rejection proof.
#[derive(Debug)]
pub struct MaterialDocumentRejected(pub anyhow::Error);
impl std::fmt::Display for MaterialDocumentRejected {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
impl std::error::Error for MaterialDocumentRejected {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.0.as_ref())
    }
}
pub(super) fn before_commit(error: anyhow::Error) -> anyhow::Error {
    if error.downcast_ref::<sqlx::Error>().is_some()
        || error
            .downcast_ref::<super::ReceiptValueAuthorizationError>()
            .is_some()
        || error
            .downcast_ref::<super::ProductionReleaseError>()
            .is_some()
    {
        error
    } else {
        anyhow::Error::new(MaterialDocumentRejected(error))
    }
}

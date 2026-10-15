"""Promote prepared source only after the previous suite closes its source window."""
import hashlib
import json
from pathlib import Path

root = Path(__file__).resolve().parents[2]
qa = root / 'qa/acceptance'
manifest = json.loads((qa / 'free-erp-inventory-consumers-cost-labor-oct8-full-after-source.json').read_text(encoding='utf8'))['files']

def change(path, old, new):
    p = root / path
    data = p.read_bytes()
    assert hashlib.sha256(data).hexdigest() == manifest[path], 'source changed since previous window: '+path
    # Preserve root file line endings and all unrelated contents.
    ending = '\r\n' if b'\r\n' in data else '\n'
    old, new = old.replace('\n', ending), new.replace('\n', ending)
    assert data.count(old.encode()) == 1, path
    p.write_bytes(data.replace(old.encode(), new.encode(), 1))

for source, destination in [
    ('cost-read-repo-oct8.draft','crates/erp-repo/src/shared_contract/cost_read.rs'),
    ('cost-read-api-oct8.draft','crates/erp-api/src/manufacturing/cost_read.rs'),
    ('cost-read-indexes-oct8.draft','migrations/0307_cost_read_indexes.sql'),
]:
    p = root / destination
    assert not p.exists(), destination
    p.write_bytes((qa/source).read_bytes())

change('crates/erp-repo/src/shared_contract.rs', 'mod cost_labor;', 'mod cost_labor;\nmod cost_read;')
path = 'crates/erp-repo/src/shared_contract/cost_commit.rs'
old = '''pub(super) async fn authorize(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    order: Uuid,
    actor: CostValueActor,
) -> anyhow::Result<Permit> {
'''
new = old + '''    authorize_permissions(tx, tenant, order, actor, &["manufacturing.write", "MFG.PRODUCTION.WRITE"]).await
}

pub(super) async fn authorize_read(
    tx: &mut Transaction<'_, Postgres>, tenant: TenantId, order: Uuid, actor: CostValueActor,
) -> anyhow::Result<Permit> {
    authorize_permissions(tx, tenant, order, actor, &["manufacturing.read", "MFG.PRODUCTION.READ"]).await
}

async fn authorize_permissions(
    tx: &mut Transaction<'_, Postgres>, tenant: TenantId, order: Uuid, actor: CostValueActor,
    permissions: &[&str],
) -> anyhow::Result<Permit> {
'''
# Combine both substitutions under one manifest comparison.
p = root / path
data = p.read_bytes()
assert hashlib.sha256(data).hexdigest() == manifest[path]
ending = '\r\n' if b'\r\n' in data else '\n'
old, new = old.replace('\n', ending), new.replace('\n', ending)
assert data.count(old.encode()) == 1
data = data.replace(old.encode(), new.encode(),1)
line = '    let permissions = ["manufacturing.write", "MFG.PRODUCTION.WRITE"];'+ending
assert data.count(line.encode()) == 1
data = data.replace(line.encode(),b'',1).replace(b'&permissions',b'permissions')
p.write_bytes(data)
change('crates/erp-api/src/manufacturing.rs','mod cost_center_binding;', 'mod cost_center_binding;\nmod cost_read;')
# This root was just updated. Do not reuse the previous manifest after this point.
p=root/'crates/erp-api/src/manufacturing.rs'
data=p.read_bytes(); ending=b'\r\n' if b'\r\n' in data else b'\n'
old=b'        .route("/v1/tenants/:tenant_id/manufacturing/work-orders/:work_order_id/cost-center-binding",post(cost_center_binding::post))'
assert data.count(old)==1
data=data.replace(old,old+ending+b'        .route("/v1/tenants/:tenant_id/manufacturing/work-orders/:work_order_id/costs/:cost_run_id",get(cost_read::read))',1)
p.write_bytes(data)
p=root/'crates/erp-api/tests/free_erp_acceptance/source_dispositions.rs'
assert hashlib.sha256(p.read_bytes()).hexdigest() == manifest[p.relative_to(root).as_posix()]
with p.open('ab') as f:f.write((qa/'cost-read-tests-oct8.draft').read_bytes())
print('cost evidence read implementation promoted after prior source window')

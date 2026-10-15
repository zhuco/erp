"""Real second QA password/session writer; no token/credential artifacts."""
import json
import runpy
import urllib.request
import uuid
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
helpers = runpy.run_path(str(ROOT / 'qa/acceptance/free-erp-product-web-fixture.py'))
fixture = json.loads(helpers['MANIFEST'].read_text(encoding='utf-8'))
assert helpers['query'](f"SELECT name FROM tenants WHERE tenant_id={helpers['literal'](fixture['tenant_id'])};") == 'QA-PRODUCT-WEB-OCT8'
output = ROOT / 'output/playwright/pending-ui/peer-write.json'
staged = ROOT / 'output/playwright/pending-ui/peer-write-request.json'
if output.exists():
    raise SystemExit('refusing to repeat a peer write with a new identity')


def request(path, body=None, token=None):
    headers = {'content-type': 'application/json'}
    if token:
        headers['authorization'] = 'Bearer ' + token
    encoded = json.dumps(body).encode() if body is not None else None
    with urllib.request.urlopen(urllib.request.Request(fixture['api'] + path, data=encoded, headers=headers), timeout=15) as response:
        return json.load(response)


login = request('/auth/login', {'tenant_id': fixture['tenant_id'], 'email': fixture['email'],
                              'password': helpers['PASSWORD'], 'client_kind': 'WEB', 'client_label': 'QA-owned-peer-version-check'})
token = login['access_token']
identity = request('/me', token=token)
assert identity['user']['user_id'] == fixture['user_id'] and identity['user']['tenant_id'] == fixture['tenant_id']
path = f"/v1/tenants/{fixture['tenant_id']}/products/{fixture['product_id']}"
if staged.exists():
    command = json.loads(staged.read_text(encoding='utf-8'))
    assert command['source'] == fixture['product_id'] and command['action'] == 'products.update'
    assert command['values'] == {'1': 'QA UI peer update'}
else:
    product = request(path, token=token)
    assert product['product_id'] == fixture['product_id']
    command = {'operation_id': str(uuid.uuid4()), 'action': 'products.update', 'source': fixture['product_id'],
               'expected_version': product['updated_at'], 'values': {'1': 'QA UI peer update'}}
    staged.write_text(json.dumps(command, indent=2) + '\n', encoding='utf-8')
receipt = request(f"/v1/tenants/{fixture['tenant_id']}/products/actions", command, token)
output.write_text(json.dumps({'session_id': login.get('session_id'), 'command': command, 'receipt': receipt}, indent=2) + '\n', encoding='utf-8')
assert receipt['outcome']['Applied']['row'] == fixture['product_id']
print('Owned peer actual password/session write applied; token kept only in memory')

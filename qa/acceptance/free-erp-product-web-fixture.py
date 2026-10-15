"""Own product UI fixture on the complete fresh-install QA baseline."""
import json
import subprocess
import uuid
from pathlib import Path
from argon2 import PasswordHasher

ROOT = Path(__file__).resolve().parents[2]
MANIFEST = ROOT / 'qa/acceptance/free-erp-product-web-oct8-fixture.json'
DSN = 'postgresql://erp_grid_test@127.0.0.1:15438/erp_free_ui_acceptance'
PASSWORD = 'ERP-owned-QA-real-password-login-2026!'


def literal(text):
    return "'" + text.replace("'", "''") + "'"


def query(sql):
    guard = """DO $$ BEGIN
    IF current_database()<>'erp_free_ui_acceptance' OR current_user<>'erp_grid_test'
      OR inet_server_addr()<>'127.0.0.1'::inet OR inet_server_port()<>15438 THEN
      RAISE EXCEPTION 'complete UI QA database identity required';
    END IF;
    IF (SELECT shobj_description(oid,'pg_database') FROM pg_database WHERE datname=current_database())
      IS DISTINCT FROM 'ERP owned free UI QA baseline 2026-10-08; no production' THEN
      RAISE EXCEPTION 'QA database marker required';
    END IF; END $$;
    """
    result = subprocess.run(['G:/PostgreSQL/17/bin/psql.exe', '-X', '-qAt', '-v', 'ON_ERROR_STOP=1', '-d', DSN],
                            input='BEGIN;\n' + guard + sql + '\nCOMMIT;', encoding='utf-8', capture_output=True)
    if result.returncode:
        raise RuntimeError(result.stderr)
    return result.stdout.strip()


def main():
    if MANIFEST.exists():
        saved = json.loads(MANIFEST.read_text(encoding='utf-8'))
        assert query(f"SELECT name FROM tenants WHERE tenant_id={literal(saved['tenant_id'])};") == 'QA-PRODUCT-WEB-OCT8'
        print('Verified existing own fixture', saved['tenant_id'])
        return
    fixture = {key: str(uuid.uuid4()) for key in ['tenant_id', 'user_id', 'role_id', 'product_id', 'other_product_id', 'sku_id']}
    fixture.update(email=fixture['user_id'] + '@product-web.qa.invalid',
                   product_code='QA-UI-' + uuid.uuid4().hex[:8], original_name='QA UI original product',
                   edited_name='QA UI saved product', sku_code='QA-UI-SKU', api='http://127.0.0.1:15441')
    q = {key: literal(value) for key, value in fixture.items()}
    password_hash = literal(PasswordHasher(time_cost=2, memory_cost=19456, parallelism=1).hash(PASSWORD))
    query(f"""
    INSERT INTO tenants(tenant_id,name) VALUES({q['tenant_id']},'QA-PRODUCT-WEB-OCT8');
    INSERT INTO users(tenant_id,user_id,email,password_hash) VALUES({q['tenant_id']},{q['user_id']},{q['email']},{password_hash});
    INSERT INTO roles(tenant_id,role_id,name) VALUES({q['tenant_id']},{q['role_id']},'QA product owner');
    INSERT INTO user_roles(tenant_id,user_id,role_id) VALUES({q['tenant_id']},{q['user_id']},{q['role_id']});
    INSERT INTO role_permissions(tenant_id,role_id,perm_code) VALUES
     ({q['tenant_id']},{q['role_id']},'skus.read'),({q['tenant_id']},{q['role_id']},'skus.write');
    INSERT INTO products(tenant_id,product_id,product_code,name,brand,category,status)
     VALUES({q['tenant_id']},{q['product_id']},{q['product_code']},{q['original_name']},'QA UI brand','QA UI category','ACTIVE'),
           ({q['tenant_id']},{q['other_product_id']},{literal(fixture['product_code']+'-B')},'QA UI untouched product','QA UI brand','QA UI category','INACTIVE');
    INSERT INTO skus(tenant_id,product_id,sku_id,sku_code,name,cost_cent)
     VALUES({q['tenant_id']},{q['product_id']},{q['sku_id']},{q['sku_code']},'QA UI SKU',230);
    """)
    MANIFEST.write_text(json.dumps(fixture, indent=2) + '\n', encoding='utf-8')
    print('Created one owned product UI fixture', fixture['tenant_id'])


if __name__ == '__main__':
    main()

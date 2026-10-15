"""Own fixture for a real loopback Web/API acceptance; never touches older tenants."""
import json
import subprocess
import uuid
from pathlib import Path
from argon2 import PasswordHasher

ROOT = Path(__file__).resolve().parents[2]
MANIFEST = ROOT / 'qa/acceptance/free-erp-trade-web-oct8-fixture.json'
DATABASE = 'postgresql://erp_grid_test@127.0.0.1:15438/erp_shared_contract_test'
PSQL = 'G:/PostgreSQL/17/bin/psql.exe'
PASSWORD = 'ERP-owned-QA-real-password-login-2026!'


def query(sql):
    guard = """DO $$ BEGIN
    IF current_database() <> 'erp_shared_contract_test'
       OR current_user <> 'erp_grid_test' OR inet_server_port() <> 15438 THEN
       RAISE EXCEPTION 'QA database identity mismatch';
    END IF; END $$;
    """
    result = subprocess.run([PSQL, '-X', '-qAt', '-v', 'ON_ERROR_STOP=1', '-d', DATABASE],
                            input='BEGIN;\n' + guard + sql + '\nCOMMIT;\n',
                            encoding='utf-8', capture_output=True, check=True)
    return result.stdout.strip()


def literal(value):
    return "'" + value.replace("'", "''") + "'"


def main():
    if MANIFEST.exists():
        fixture = json.loads(MANIFEST.read_text(encoding='utf-8'))
        assert query(f"SELECT name FROM tenants WHERE tenant_id={literal(fixture['tenant_id'])};") == 'QA-TRADE-WEB-OCT8'
        query(f"INSERT INTO role_permissions(tenant_id,role_id,perm_code) VALUES({literal(fixture['tenant_id'])},{literal(fixture['role_id'])},'skus.read') ON CONFLICT DO NOTHING;")
        print('Verified existing own Web fixture:', fixture['tenant_id'])
        return
    fixture = {key: str(uuid.uuid4()) for key in ['tenant_id', 'user_id', 'role_id', 'warehouse_id', 'sku_id', 'partner_id']}
    fixture.update(email=fixture['user_id'] + '@trade-web.qa.invalid',
                   customer_code='QA-WEB-CUSTOMER', sku_code='QA-WEB-SKU',
                   document_no='QA-WEB-' + uuid.uuid4().hex[:12],
                   api='http://127.0.0.1:15441', web='http://127.0.0.1:15440/?erp')
    q = {key: literal(value) for key, value in fixture.items()}
    password_hash = literal(PasswordHasher(time_cost=2, memory_cost=19456, parallelism=1).hash(PASSWORD))
    permissions = ['orders.read', 'orders.write', 'orders:read', 'products:read', 'skus:read', 'skus.read']
    sql = f"""
    INSERT INTO tenants(tenant_id,name) VALUES({q['tenant_id']},'QA-TRADE-WEB-OCT8');
    INSERT INTO users(tenant_id,user_id,email,password_hash,status)
      VALUES({q['tenant_id']},{q['user_id']},{q['email']},{password_hash},'ACTIVE');
    INSERT INTO roles(tenant_id,role_id,name) VALUES({q['tenant_id']},{q['role_id']},'QA-WEB-owner');
    INSERT INTO user_roles(tenant_id,user_id,role_id) VALUES({q['tenant_id']},{q['user_id']},{q['role_id']});
    INSERT INTO warehouses(tenant_id,warehouse_id,name,code)
      VALUES({q['tenant_id']},{q['warehouse_id']},'QA Web warehouse','QA-WEB-WH');
    INSERT INTO user_warehouse_access(tenant_id,user_id,warehouse_id)
      VALUES({q['tenant_id']},{q['user_id']},{q['warehouse_id']});
    INSERT INTO skus(tenant_id,sku_id,sku_code) VALUES({q['tenant_id']},{q['sku_id']},{q['sku_code']});
    INSERT INTO shared_partners(tenant_id,partner_id,name,customer,supplier,customer_no,term_days,status)
      VALUES({q['tenant_id']},{q['partner_id']},'QA Web customer',true,false,{q['customer_code']},30,'ACTIVE');
    """
    sql += '\n'.join(f"INSERT INTO role_permissions(tenant_id,role_id,perm_code) VALUES({q['tenant_id']},{q['role_id']},{literal(p)});" for p in permissions)
    query(sql)
    MANIFEST.write_text(json.dumps(fixture, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    print('Created one owned QA Web fixture:', fixture['tenant_id'])


if __name__ == '__main__':
    main()

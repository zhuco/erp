"""Exercise the new-table path and owner rejection in a rolled-back owned schema."""
import hashlib
import json
import runpy
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
helpers = runpy.run_path(str(ROOT / 'qa/acceptance/free-erp-product-web-fixture.py'))
source = ROOT / 'migrations/0310_formal_password_recovery.sql'
sql = """
DO $$ BEGIN
 IF to_regnamespace('qa_password_recovery_install_trial') IS NOT NULL THEN
  RAISE EXCEPTION 'owned trial schema already exists; refusing to alter it';
 END IF;
END $$;
CREATE SCHEMA qa_password_recovery_install_trial;
SET LOCAL search_path=qa_password_recovery_install_trial,pg_catalog;
CREATE TABLE users(tenant_id uuid NOT NULL,user_id uuid PRIMARY KEY);
""" + source.read_text(encoding='utf-8') + """
INSERT INTO users VALUES('11111111-1111-4111-8111-111111111111','22222222-2222-4222-8222-222222222222');
INSERT INTO password_reset_tokens(tenant_id,user_id,token_hash,expires_at)
 VALUES('11111111-1111-4111-8111-111111111111','22222222-2222-4222-8222-222222222222','QA-owner-token',now()+interval '1 hour');
DO $$ BEGIN
 BEGIN
  INSERT INTO password_reset_tokens(tenant_id,user_id,token_hash,expires_at)
   VALUES('33333333-3333-4333-8333-333333333333','22222222-2222-4222-8222-222222222222','QA-foreign-token',now()+interval '1 hour');
  RAISE EXCEPTION 'foreign token owner accepted';
 EXCEPTION WHEN foreign_key_violation THEN NULL;
 END;
END $$;
SELECT json_build_object('created_namespace',(SELECT n.nspname FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace WHERE c.oid='password_reset_tokens'::regclass),
 'valid_rows',(SELECT count(*) FROM password_reset_tokens),
 'owner_constraint',(SELECT count(*) FROM pg_constraint WHERE conrelid='password_reset_tokens'::regclass AND conname='password_reset_token_user_owner'),
 'owner_index',to_regclass('users_password_recovery_owner_key') IS NOT NULL,
 'unused_index',to_regclass('password_reset_tokens_unused_owner') IS NOT NULL);
ROLLBACK;
"""
# The guard and first BEGIN are supplied by the fixed QA helper. Explicit ROLLBACK
# removes only trial objects; its trailing COMMIT then has no active transaction.
result = json.loads(helpers['query'](sql))
assert result == {'created_namespace': 'qa_password_recovery_install_trial', 'valid_rows': 1,
                  'owner_constraint': 1, 'owner_index': True, 'unused_index': True}
assert helpers['query']("SELECT to_regnamespace('qa_password_recovery_install_trial') IS NULL;") == 't'
report = {'status': 'PASS', 'source_sha256': hashlib.sha256(source.read_bytes()).hexdigest(),
          'result': result, 'trial_objects_remain': False,
          'scope': 'new 0310 table creation and mismatched-owner rejection; not full fresh 319 installation or old token validation'}
(ROOT / 'qa/acceptance/free-erp-password-recovery-migration-trial.json').write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
print('0310 new-table creation, real owner rejection and rollback preservation PASS')

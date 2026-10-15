"""Create guarded, immutable QA capture controls; no database or build mutations."""
from pathlib import Path

root=Path(__file__).resolve().parents[2]
base=root/"qa/acceptance"
sql=(base/"free-erp-production-release-readback.sql").read_text(encoding="utf8")
sql=sql.replace("'release_commands_present',to_regclass('public.mfg_receipt_release_commands') is not null", "'release_commands_present',to_regclass('public.mfg_receipt_release_commands') is not null, 'inspection_bindings_present',to_regclass('public.shared_inspection_source_bindings') is not null")
sql=sql.replace("'mfg_receipt_release_commands','mfg_receipt_release_effects']) name", "'mfg_receipt_release_commands','mfg_receipt_release_effects','shared_inspection_templates','shared_inspection_template_lines','shared_inspection_source_bindings','shared_sales_revisions','shared_sales_lines','mfg_quality_releases']) name")
capture=(base/"free-erp-production-release-capture.py").read_text(encoding="utf8").replace("free-erp-production-release-", "free-erp-production-quality-source-")
for filename,content in [("free-erp-production-quality-source-readback.sql",sql),("free-erp-production-quality-source-capture.py",capture)]:
    with (base/filename).open("x",encoding="utf8",newline="\n") as stream:
        stream.write(content)
print("immutable quality source capture controls saved")

"""Preserve false doc-gate failure caused by POST-after-GET router parsing."""
from pathlib import Path

base = Path(__file__).resolve().parent
archives = {
    'free-erp-specification-docs-verify.py': 'free-erp-specification-docs-verify-before-chained-methods.py.txt',
    'free-erp-specification-docs.json': 'free-erp-specification-docs-chained-methods-parser-failed.json',
    'free-erp-fulfillment-atomic-repair.json': 'free-erp-fulfillment-atomic-repair-doc-gate-failed.json',
}
for name, archived in archives.items():
    source, destination = base / name, base / archived
    if destination.exists():
        raise SystemExit('refusing doc-gate history overwrite')
    if name == 'free-erp-fulfillment-atomic-repair.json':
        source.rename(destination)
    else:
        destination.write_bytes(source.read_bytes())
print('preserved original parser, failed doc report and rejected delivery binding')

"""Keep prior reports immutable and verify the final call-site cleanup identically."""
from pathlib import Path

base = Path(__file__).resolve().parent
source = (base / 'free-erp-fulfillment-atomic-verify-v3.py').read_text(encoding='utf8')
source = source.replace("f'free-erp-fulfillment-{args.stage}-facts-v3.json'", "f'free-erp-fulfillment-{args.stage}-facts-v4.json'")
source = source.replace("if args.stage == 'atomic-final':", "if args.stage in ('atomic-final', 'atomic-current'):")
source = source.replace("if args.stage in ('atomic-full', 'atomic-final'):", "if args.stage in ('atomic-full', 'atomic-final', 'atomic-current'):")
destination = base / 'free-erp-fulfillment-atomic-verify-v4.py'
locations = (base / 'free-erp-fulfillment-atomic-locations.py').read_text(encoding='utf8')
locations = locations.replace('free-erp-fulfillment-atomic-document-locations.json', 'free-erp-fulfillment-atomic-current-document-locations.json')
location_script = base / 'free-erp-fulfillment-atomic-locations-current.py'
if destination.exists() or location_script.exists():
    raise SystemExit('refusing final verifier overwrite')
destination.write_text(source, encoding='utf8')
location_script.write_text(locations, encoding='utf8')
print('created immutable current-source verifiers with the same exact-time/fault/competition assertions')

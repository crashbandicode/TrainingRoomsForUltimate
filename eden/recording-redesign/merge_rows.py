#!/usr/bin/env python3
"""Add new rows and segments to a live run's rows.json without losing the sha256s it recorded.

python3 -I merge_rows.py <run rows.json> <new docs-gif-rows.json> [skip-id,skip-id]
"""
import json
import sys
from pathlib import Path

live_path, new_path = Path(sys.argv[1]), Path(sys.argv[2])
live, new = json.loads(live_path.read_text()), json.loads(new_path.read_text())
added_segments = [name for name in new['segments'] if name not in live['segments']]
for name in added_segments:
    live['segments'][name] = new['segments'][name]
have = {row['id'] for row in live['rows']} | set(sys.argv[3].split(',') if len(sys.argv) > 3 else [])
added_rows = [row for row in new['rows'] if row['id'] not in have]
live['rows'].extend(added_rows)
live_path.write_text(json.dumps(live, indent=2) + '\n')
print(json.dumps({'segments': added_segments, 'rows': [row['id'] for row in added_rows]}))

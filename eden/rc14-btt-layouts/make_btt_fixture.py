"""The seeded saved layout for btt-E42 (btt-5, 2026-11-14; btt-3's slot file targets/2.json before): one file in `layouts/`,
`seeded-big-battlefield.json`, three targets on Big Battlefield's floor in the format of `model::btt::Layout` (a store
record: schema_version 1, its name, the stage, its use number, 1 to 10 targets). The boot reads it when the stage opens;
nothing in the boot writes it before btt-E42 loads it.

The stage id is the owned stage table's (`model::stages::lobby_stages::STAGES`: Battlefield 0, Big Battlefield 2), and
btt-st-bbf checks it against the game's own `[PracticeStageRequest] queued id=` line first, so a wrong id fails that row.

`validate()` mirrors `Layout::validate` (schema 1, a name of 1 to 48 characters, a stage >= 0, 1 to 10 finite pairs), so the
fixture is checked offline. It is written beside this script and pinned in the run's config by sha256.
"""
import hashlib
import json
import math
from pathlib import Path

HERE = Path(__file__).resolve().parent
FILE_ID, NAME, STAGE_ID = 'seeded-big-battlefield', 'Seeded · Big Battlefield', 2
TARGETS = [[-40.0, 10.0], [0.0, 10.0], [40.0, 10.0]]


def build():
    return {'schema_version': 1, 'name': NAME, 'stage': STAGE_ID, 'used': 1, 'targets': TARGETS}


def validate(text):
    v = json.loads(text)
    assert v['schema_version'] == 1 and isinstance(v['stage'], int) and v['stage'] >= 0, 'schema or stage'
    assert 0 < len(v['name'].strip()) <= 48, 'name'
    assert 1 <= len(v['targets']) <= 10 and all(len(p) == 2 and all(math.isfinite(c) for c in p) for p in v['targets']), 'targets'
    return v


def main():
    out = HERE / 'fixtures' / 'btt'
    out.mkdir(parents=True, exist_ok=True)
    text = json.dumps(build(), ensure_ascii=False, separators=(',', ':')) + '\n'
    validate(text)
    path = out / ('%s.json' % FILE_ID)
    path.write_text(text, encoding='utf-8')
    print(path, hashlib.sha256(text.encode()).hexdigest())


if __name__ == '__main__':
    main()

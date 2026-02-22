import re, subprocess
from pathlib import Path
html = Path('web/src/legacy/template.html').read_text(encoding='utf-8', errors='ignore')
ids = re.findall(r'id="([^"]+)"', html)
for id_name in ids:
    proc = subprocess.run(
        ['rg', '-uu', '-n', id_name, 'web'],
        capture_output=True,
        text=True,
        encoding='utf-8',
        errors='ignore',
    )
    lines = [l for l in proc.stdout.splitlines() if 'template.html' not in l]
    if not lines:
        print(id_name)

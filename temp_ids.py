import re
from pathlib import Path
text=Path('web/src/legacy/template.html').read_text(encoding='utf-8')
ids=re.findall(r'id="([^\"]+)"',text)
print(len(ids))
for idv in sorted(set(ids)):
    print(idv)

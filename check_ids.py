import pathlib, re
text = pathlib.Path('web/src/legacy/template.html').read_text(encoding='utf-8', errors='ignore')
ids = sorted(set(re.findall(r'id="([^"]+)"', text)))
files = [p for p in pathlib.Path('.').rglob('*') if p.is_file() and p != pathlib.Path('web/src/legacy/template.html')]
unused = []
for id_ in ids:
    if id_ == 'legacyCompatibilityRoot':
        continue
    found = False
    for path in files:
        try:
            data = path.read_text(encoding='utf-8', errors='ignore')
        except Exception:
            continue
        if id_ in data:
            found = True
            break
    if not found:
        unused.append(id_)
print('unused', len(unused))
for id_ in unused[:30]:
    print(id_)

from pathlib import Path
import re
text=Path('web/src/legacy/template.html').read_text(encoding='utf-8')
ids=sorted(set(re.findall(r'id="([^"]+)"', text)))
paths={
    'app': Path('web/src/legacy/app.js'),
    'bridges': list(Path('web/src/legacy/bridges').rglob('*')) if Path('web/src/legacy/bridges').exists() else [],
    'components': list(Path('web/src/components').rglob('*')) if Path('web/src/components').exists() else [],
    'stores': list(Path('web/src/stores').rglob('*')) if Path('web/src/stores').exists() else [],
}
texts={}
texts['app']=paths['app'].read_text(encoding='utf-8')
for key in ('bridges','components','stores'):
    txts=[]
    for p in paths[key]:
        if p.is_file():
            try:
                txts.append(p.read_text(encoding='utf-8'))
            except UnicodeDecodeError:
                try:
                    txts.append(p.read_text(encoding='latin1'))
                except:
                    pass
    texts[key]='\n'.join(txts)

only_app=[]
for idv in ids:
    in_app = idv in texts['app']
    in_bridges = idv in texts['bridges']
    in_components = idv in texts['components']
    in_stores = idv in texts['stores']
    if in_app and not (in_bridges or in_components or in_stores):
        only_app.append(idv)
print(len(only_app))
for idv in sorted(only_app):
    print(idv)

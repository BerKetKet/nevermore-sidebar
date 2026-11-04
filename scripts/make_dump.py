import os
import glob

base_dir = '/home/bgg/projects/nevermore-sidebar'
out_file = os.path.join(base_dir, 'docs/04_FULL_SOURCE_CODE_DUMP_Phase-7_5_1.md')

files_to_dump = [
    'Cargo.toml',
    'DEPENDENCIES.md',
    'AGENTS.md',
    'docs/EVIDENCE_LOG.md',
    'docs/PLUGIN_SDK.md',
    'docs/THEMING_GUIDE.md',
]

for spk in sorted(glob.glob(os.path.join(base_dir, 'docs/spikes/*.md'))):
    files_to_dump.append(os.path.relpath(spk, base_dir))

for rs_file in sorted(glob.glob(os.path.join(base_dir, 'crates/**/*.rs'), recursive=True)):
    files_to_dump.append(os.path.relpath(rs_file, base_dir))

with open(out_file, 'w', encoding='utf-8') as out:
    out.write('# NEVERMORE SIDEBAR — PHASE 7.5.1 COMPLETE SOURCE CODE DUMP\n\n')
    out.write('**Commit SHA:** `822dde1` (`822dde1f893a4f87be1bce9acfd9ba52f0399d37`)\n')
    out.write('**Date:** 2026-11-04\n')
    out.write('**Repository:** https://github.com/BerKetKet/nevermore-sidebar\n\n')
    out.write('This snapshot contains all core documents, configuration files, spike reports, and Rust source code for the Nevermore Sidebar following the Phase 7.5.1 remediation pass.\n\n---\n\n')

    for rel_path in files_to_dump:
        abs_path = os.path.join(base_dir, rel_path)
        if not os.path.exists(abs_path):
            continue
        ext = rel_path.split('.')[-1]
        lang = 'rust' if ext == 'rs' else ('toml' if ext == 'toml' else 'markdown')
        out.write(f'## File: `{rel_path}`\n\n```{lang}\n')
        with open(abs_path, 'r', encoding='utf-8', errors='replace') as f:
            content = f.read()
            out.write(content)
            if not content.endswith('\n'):
                out.write('\n')
        out.write('```\n\n---\n\n')

print('Dump written, size:', os.path.getsize(out_file))

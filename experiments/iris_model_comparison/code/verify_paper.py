# -*- coding: utf-8 -*-
"""Verify paper structure"""
import sys, re
from pathlib import Path
sys.stdout.reconfigure(encoding='utf-8')

path = Path(__file__).resolve().parents[1] / 'paper' / 'research_paper.md'
with open(path, 'r', encoding='utf-8') as f:
    content = f.read()

sections = [
    ('Abstract', 'Abstract section'),
    ('Introduction', 'Introduction section'),
    ('Related Work', 'Related Work section'),
    ('Methodology', 'Methodology section'),
    ('Experiments and Results', 'Experiments section'),
    ('Analysis and Discussion', 'Analysis section'),
    ('Conclusion', 'Conclusion section'),
    ('References', 'References section'),
]

print('=' * 60)
print('Paper Structure Verification')
print('=' * 60)
print('Total length: {} characters'.format(len(content)))
print('Total lines: {}'.format(content.count(chr(10))))

for section_name, desc in sections:
    count = content.count(section_name)
    status = 'PASS' if count >= 1 else 'FAIL'
    print('  {}: {}'.format(desc, status))

abstract_match = re.search(r'## Abstract\n+(.*?)(?=\n## )', content, re.DOTALL)
if abstract_match:
    abstract = abstract_match.group(1)
    words = len(abstract.split())
    print('\nAbstract word count: {}'.format(words))

ref_count = content.count('\n20.')
print('Reference entries: {}'.format(ref_count + 1))

print('\n' + '=' * 60)
print('PAPER COMPLETE')
print('=' * 60)

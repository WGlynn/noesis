#!/usr/bin/env python3
"""Render the UKIPO description-only filing to .docx — Arial 11, 1.5 spacing.

Reads UKIPO-FILING.md, strips the internal draft/strategy header (everything before
'## INVENTION TITLE'), and emits a clean specification document for upload.
"""
import re
from docx import Document
from docx.shared import Pt, RGBColor
from docx.enum.text import WD_ALIGN_PARAGRAPH

SRC = "UKIPO-FILING.md"
OUT = r"C:\Users\Will\Desktop\Noesis-UKIPO-description.docx"


def clean(text: str) -> str:
    text = re.sub(r"\*\*(.+?)\*\*", r"\1", text)  # bold
    text = re.sub(r"(?<!\*)\*(?!\*)(.+?)\*", r"\1", text)  # italic
    text = text.replace("`", "")
    return text.strip()


lines = open(SRC, encoding="utf-8").read().splitlines()
# drop everything before the invention title
start = next(i for i, l in enumerate(lines) if l.strip() == "## INVENTION TITLE")
lines = lines[start:]

doc = Document()

# base style: Arial 11, 1.5 spacing
normal = doc.styles["Normal"]
normal.font.name = "Arial"
normal.font.size = Pt(11)
normal.paragraph_format.line_spacing = 1.5
normal.paragraph_format.space_after = Pt(6)

for h, sz in (("Heading 1", 13), ("Heading 2", 11.5)):
    st = doc.styles[h]
    st.font.name = "Arial"
    st.font.size = Pt(sz)
    st.font.bold = True
    st.font.color.rgb = RGBColor(0, 0, 0)
    st.paragraph_format.line_spacing = 1.5

i = 0
while i < len(lines):
    line = lines[i].rstrip()
    if line.strip() == "## INVENTION TITLE":
        # next non-empty line is the title
        j = i + 1
        while j < len(lines) and not lines[j].strip():
            j += 1
        title = clean(lines[j])
        p = doc.add_paragraph()
        p.alignment = WD_ALIGN_PARAGRAPH.CENTER
        run = p.add_run(title)
        run.font.name = "Arial"
        run.font.size = Pt(14)
        run.font.bold = True
        p.paragraph_format.line_spacing = 1.5
        i = j + 1
        continue
    if line.startswith("### "):
        doc.add_heading(clean(line[4:]), level=2)
    elif line.startswith("## "):
        doc.add_heading(clean(line[3:]), level=1)
    elif line.strip():
        doc.add_paragraph(clean(line))
    i += 1

doc.save(OUT)
print(f"wrote {OUT}")

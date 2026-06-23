#!/usr/bin/env python3
"""Create 4-up booklet for double-fold (no cutting required).

Layout:
  4 | 1 
--+--
 3 | 2

Pages 2 and 3 are rotated 180° so a single sheet can be folded twice
(horizontal then vertical) to create a 4-page booklet.
"""

from PyPDF2 import PdfReader, PdfWriter
import sys
import subprocess

INPUT = "../generated/quarter/guest-schedule-quarter.pdf"
OUTPUT = "scratch/temp_reordered.pdf"
FINAL_OUTPUT = "../generated/letter/guest-schedule-quarter-4up-folded.pdf"

# If input file specified as argument, use it
if len(sys.argv) > 1:
    INPUT = sys.argv[1]
if len(sys.argv) > 2:
    FINAL_OUTPUT = sys.argv[2]

reader = PdfReader(INPUT)
writer = PdfWriter()

total_pages = len(reader.pages)
if total_pages % 4 != 0:
    print(f"Warning: Total pages ({total_pages}) is not a multiple of 4")

# Handle special cases
if total_pages % 4 != 0:
    # Add blank pages to make it a multiple of 4
    blank_pages_needed = 4 - (total_pages % 4)
    print(f"Adding {blank_pages_needed} blank pages to make multiple of 4")
    effective_pages = total_pages + blank_pages_needed
else:
    effective_pages = total_pages

num_booklets = effective_pages // 4
num_4up_pages = num_booklets  # Each 4-up page creates 1 booklet (no cutting)

print(f"Total pages: {total_pages}")
print(f"Number of booklets: {num_booklets}")
print(f"Number of 4-up output pages: {num_4up_pages}")

pages = []
rotate_indices = []  # 0-based indices in the output that need rotation
for booklet_num in range(num_4up_pages):
    booklet_start = booklet_num * 4 + 1
    
    # Layout: 4 | 1 on top, 3 | 2 on bottom (with 2 and 3 rotated)
    pages.extend([booklet_start + 3, booklet_start, booklet_start + 2, booklet_start + 1])
    rotate_indices.extend([len(pages) - 2, len(pages) - 1])  # Last two added (bottom positions)

# Add pages in the correct order, rotating as needed
for i, page_num in enumerate(pages):
    if page_num > total_pages:
        # Add blank page for padding
        first_page = reader.pages[0]
        blank_page = writer.add_blank_page(
            width=first_page.mediabox.width,
            height=first_page.mediabox.height
        )
        if i in rotate_indices:
            blank_page.rotate(180)
        continue
    else:
        actual_page_num = page_num
    
    page = reader.pages[actual_page_num - 1]  # Convert to 0-based
    if i in rotate_indices:
        page = page.rotate(180)
    writer.add_page(page)

# Save the reordered PDF
with open(OUTPUT, "wb") as f:
    writer.write(f)

print(f"Created {OUTPUT} with {len(pages)} pages")
print(f"Rotated {len(rotate_indices)} pages")

# Now use pdfjam to create 4-up layout
print("Creating 4-up layout with pdfjam...")
result = subprocess.run([
    'pdfjam', '--nup', '2x2', '--paper', 'letter',
    '--outfile', FINAL_OUTPUT, OUTPUT
], capture_output=True, text=True)

if result.returncode == 0:
    print(f"Created {FINAL_OUTPUT}")
    # Clean up temp file
    import os
    os.remove(OUTPUT)
else:
    print(f"Error creating 4-up layout: {result.stderr}")

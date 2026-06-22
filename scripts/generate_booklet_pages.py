#!/usr/bin/env python3
"""Create 4-up booklet with bottom half rotated 180 degrees."""

from PyPDF2 import PdfReader, PdfWriter
import sys
import subprocess

INPUT = "../generated/letter/guest-schedule-letter.pdf"
OUTPUT = "scratch/temp_reordered.pdf"
FINAL_OUTPUT = "../generated/letter/guest-schedule-letter-4up.pdf"

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

num_booklets = total_pages // 4
num_4up_pages = num_booklets // 2  # Each 4-up page creates 2 booklets

print(f"Total pages: {total_pages}")
print(f"Number of booklets: {num_booklets}")
print(f"Number of 4-up output pages: {num_4up_pages}")

pages = []
rotate_indices = []  # 0-based indices in the output that need rotation
for booklet_pair in range(num_4up_pages):
    booklet1_start = booklet_pair * 8 + 1
    booklet2_start = booklet_pair * 8 + 5
    
    # Front of 4-up page
    pages.extend([booklet1_start + 3, booklet1_start, booklet2_start + 3, booklet2_start])
    rotate_indices.extend([len(pages) - 2, len(pages) - 1])  # Last two added (bottom positions)
    
    # Back of 4-up page
    pages.extend([booklet1_start + 1, booklet1_start + 2, booklet2_start + 1, booklet2_start + 2])
    rotate_indices.extend([len(pages) - 2, len(pages) - 1])  # Last two added (bottom positions)

# Add pages in the correct order, rotating as needed
for i, page_num in enumerate(pages):
    page = reader.pages[page_num - 1]  # Convert to 0-based
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

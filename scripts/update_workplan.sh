#!/bin/bash
# Update the combined docs/WORK_PLAN.md file

echo "Updating docs/WORK_PLAN.md..."
cd "$(dirname "$0")/.."
perl scripts/combine_workplans.pl
echo "Done!"

#!/bin/bash
# Update the combined WORK_PLAN.md file

echo "Updating WORK_PLAN.md..."
cd "$(dirname "$0")/.."
perl work-plan/combine_workplans.pl
echo "Done!"

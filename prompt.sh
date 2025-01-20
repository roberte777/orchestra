#!/bin/bash

# Set default ignored folders
IGNORE_FOLDERS=("build" "install" "cmake" "deps" ".vscode" "tests" ".git" ".venv_poetry" ".venv" ".cache_poetry" ".cache_pip" "__pycache__" ".cache" "target" "frontend/target" "frontend/frontend/node_modules" "bun.lockb" "vite.config.ts" "tsconfig.json" "tsconfig.node.json" "tsconfig.app.json" "Cargo.lock" "frontend/frontend/package-lock.json")

# Flag to include "test" files and folders
INCLUDE_TEST=false

# Parse command line arguments
for arg in "$@"; do
  if [ "$arg" == "-t" ]; then
    INCLUDE_TEST=true
  else
    IGNORE_FOLDERS+=("$arg")
  fi
done

# Add "test" folders and files to the ignore list if -t is not passed
if [ "$INCLUDE_TEST" = false ]; then
  IGNORE_FOLDERS+=("*test*")
fi

# ------------------------------------------------------------------------------
# Build a single "prune" expression from IGNORE_FOLDERS
#
# We'll construct something like:
#    \( ( -path "*/folder1" -o -path "*/folder2" -o ... ) -prune \) -o (the real work)
# ------------------------------------------------------------------------------
PRUNE_PATTERNS=()
for folder in "${IGNORE_FOLDERS[@]}"; do
  # Each folder becomes: -path "*/$folder" -o
  PRUNE_PATTERNS+=( -path "*/$folder" -o )
done

# Remove the trailing "-o" so we don't end with an OR at the end
unset 'PRUNE_PATTERNS[${#PRUNE_PATTERNS[@]}-1]'

# Output file
OUTPUT_FILE="prompt_script.txt"

# Write the project structure with files to the output file
{
  echo "Project Folder Structure:"
  echo "========================"

  # ----------------------------------------------------------------------------
  # FOLDER TREE
  #
  # Explanation:
  #   find . \
  #     ( (PRUNE_PATTERNS) -prune )  -o  ( -type d -print )
  #
  # This means:
  #   1. If the path matches any ignored folder, prune (skip) it.
  #   2. Otherwise (-o), if it's a directory, print its path.
  # ----------------------------------------------------------------------------
  find . \( \( "${PRUNE_PATTERNS[@]}" \) -prune \) -o \( -type d -print \) \
    | sed -e 's|[^/]*/|  |g'

  echo
  echo "Files with Contents:"
  echo "===================="

  # ----------------------------------------------------------------------------
  # FILES + CONTENT
  #
  # Similar logic, except we look for -type f with our desired -name patterns.
  # ----------------------------------------------------------------------------
  find . \( \( "${PRUNE_PATTERNS[@]}" \) -prune \) -o \
         \( -type f \
            \( -name "*.py" \
            -o -name "*.h" \
            -o -name "*.hpp" \
            -o -name "*.c" \
            -o -name "*.cpp" \
            -o -name "*.yaml" \
            -o -name "*.yml" \
            -o -name "*.json" \
            -o -name "*.toml" \
            -o -name "Dockerfile*" \
            -o -name "*.go" \
            -o -name "Makefile" \
            -o -name "*.mk" \
            -o -name "*.env" \
            -o -name "*.bat" \
            -o -name "*.rs" \
            -o -name "*.js" \
            -o -name "*.jsx" \
            -o -name "*.ts" \
            -o -name "*.tsx" \
            -o -name "*.dart" \) \
         -print \) \
    | while read -r file; do

        if [ -f "$file" ]; then
          echo
          echo "==== $file ===="
          echo
          cat "$file"
        fi
      done

} > "$OUTPUT_FILE"

# Notify user of completion
echo "Project structure and file contents written to $OUTPUT_FILE"

#!/usr/bin/env bash
# The purpose of this script is to copy all assets from the ./assets directory to the ./www/public/assets directory,
# except the contents of any ./assets/ignore directory. The script can be used as part of the WASM build process
# which allows running the game in a web browser.

# Exit immediately if a command exits with a non-zero status, if an undefined variable is used, or if any command in
# a pipeline fails
set -euo pipefail

# Set some variables
IFS=$'\n\t'
SOURCE_DIRECTORY="./assets"
DESTINATION_DIRECTORY="./www/public/assets"
DRY_RUN=false

print_usage() {
  echo "Usage: $0 [--dry-run|-n]"
  echo "  --dry-run, -n    Show what would be done without making changes"
}

# Parse args
while [[ ${#} -gt 0 ]]; do
  case "$1" in
    --dry-run|-n)
      DRY_RUN=true
      shift
      ;;
    --help|-h)
      print_usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1"
      print_usage
      exit 2
      ;;
  esac
done

cmd_prefix=""
if $DRY_RUN; then
  echo "Running in dry-run mode. No files will be changed."
  cmd_prefix="(dry-run)"
fi

echo "Copying assets from $SOURCE_DIRECTORY to $DESTINATION_DIRECTORY..."

# Verify source exists
if [[ ! -d "$SOURCE_DIRECTORY" ]]; then
  echo "Source directory does not exist: $SOURCE_DIRECTORY" >&2
  exit 1
fi

# Remove the destination directory if it exists
if [[ -d "$DESTINATION_DIRECTORY" ]]; then
  if $DRY_RUN; then
    echo "Would remove existing destination directory: $DESTINATION_DIRECTORY"
  else
    echo "Removing the existing destination directory: $DESTINATION_DIRECTORY"
    rm -rf -- "$DESTINATION_DIRECTORY"
  fi
fi

# Create the destination directory if it doesn't exist
if [[ ! -d "$DESTINATION_DIRECTORY" ]]; then
  if $DRY_RUN; then
    echo "Would create the destination directory: $DESTINATION_DIRECTORY"
  else
    mkdir -p -- "$DESTINATION_DIRECTORY"
    echo "Created the destination directory: $DESTINATION_DIRECTORY"
  fi
fi

# Copy all files and directories, excluding any path that contains /ignore/
RSYNC_OPTS=( -a --human-readable --info=progress2 --exclude='ignore/**' )
if $DRY_RUN; then
  RSYNC_OPTS+=( --dry-run --verbose )
else
  RSYNC_OPTS+=( --verbose )
fi

# Run rsync to perform the copy
rsync "${RSYNC_OPTS[@]}" -- "$SOURCE_DIRECTORY/" "$DESTINATION_DIRECTORY/"

if $DRY_RUN; then
  echo "Dry-run finished. No changes were made."
else
  echo "DONE!"
fi

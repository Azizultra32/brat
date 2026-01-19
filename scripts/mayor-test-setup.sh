#!/bin/bash
# Mayor Test Harness Setup
# Creates a test repository with sample code for testing the AI Mayor

set -e

TEST_DIR="/tmp/mayor-test-repo"
BRAT_REPO="${BRAT_REPO:-$HOME/Code/brat}"

echo "=============================================="
echo "  MAYOR TEST HARNESS SETUP"
echo "=============================================="
echo ""

# 1. Clean previous test
if [ -d "$TEST_DIR" ]; then
    echo "Removing existing test repo..."
    rm -rf "$TEST_DIR"
fi

mkdir -p "$TEST_DIR"
cd "$TEST_DIR"
echo "Created: $TEST_DIR"

# 2. Initialize git
echo ""
echo "Initializing git repository..."
git init --quiet
git config user.email "test@brat.dev"
git config user.name "Brat Test"

# 3. Create sample codebase with intentional issues
echo "Creating sample Python codebase..."
mkdir -p src/tests

cat > src/main.py << 'EOF'
"""Main entry point for the sample application."""
# TODO: Add argument parsing with argparse
# TODO: Add logging with proper log levels

from utils import process_data, format_output

def main():
    """Process some sample data."""
    data = {"items": [1, 2, 3, 4, 5]}
    result = process_data(data)
    output = format_output(result)
    print(f"Result: {output}")

if __name__ == "__main__":
    main()
EOF

cat > src/utils.py << 'EOF'
"""Utility functions for data processing."""

def process_data(data):
    """Process the input data and return the average.

    Args:
        data: Dictionary containing an 'items' key with a list of numbers.

    Returns:
        The average of the items.
    """
    # BUG: No input validation - crashes on empty list (ZeroDivisionError)
    # BUG: No type checking - crashes on non-dict input
    items = data["items"]
    return sum(items) / len(items)

def format_output(value):
    """Format a numeric value for display.

    Args:
        value: A numeric value to format.

    Returns:
        Formatted string representation.
    """
    # TODO: Add proper number formatting (thousands separator, decimal places)
    # TODO: Add support for different output formats (json, csv)
    return str(value)

def validate_data(data):
    """Validate input data structure.

    Args:
        data: Data to validate.

    Returns:
        True if valid, False otherwise.
    """
    # TODO: Implement validation logic
    pass
EOF

cat > src/config.py << 'EOF'
"""Configuration management."""
# TODO: Add configuration file support (yaml/toml)
# TODO: Add environment variable support
# TODO: Add configuration validation

DEFAULT_CONFIG = {
    "output_format": "text",
    "precision": 2,
    "verbose": False,
}

def get_config():
    """Get the current configuration."""
    # BUG: Returns mutable default - modifications persist
    return DEFAULT_CONFIG
EOF

cat > src/tests/test_utils.py << 'EOF'
"""Tests for utility functions."""
import unittest
import sys
sys.path.insert(0, '..')

from utils import process_data, format_output

class TestProcessData(unittest.TestCase):
    """Tests for process_data function."""

    def test_basic_average(self):
        """Test basic average calculation."""
        result = process_data({"items": [1, 2, 3]})
        self.assertEqual(result, 2.0)

    def test_single_item(self):
        """Test with a single item."""
        result = process_data({"items": [42]})
        self.assertEqual(result, 42.0)

    # TODO: Add test for empty list (should handle gracefully)
    # TODO: Add test for missing 'items' key
    # TODO: Add test for non-numeric items
    # TODO: Add test for None input

class TestFormatOutput(unittest.TestCase):
    """Tests for format_output function."""

    def test_integer(self):
        """Test formatting an integer."""
        result = format_output(42)
        self.assertEqual(result, "42")

    def test_float(self):
        """Test formatting a float."""
        result = format_output(3.14159)
        self.assertEqual(result, "3.14159")

    # TODO: Add test for large numbers
    # TODO: Add test for negative numbers
    # TODO: Add test for None input

if __name__ == "__main__":
    unittest.main()
EOF

# Create a README
cat > README.md << 'EOF'
# Sample Python Project

A minimal Python project for testing the brat AI Mayor.

## Known Issues

1. `process_data()` - No input validation, crashes on empty list
2. `get_config()` - Returns mutable default dictionary
3. Missing proper error handling throughout

## TODOs

- Add argument parsing to main.py
- Add logging
- Implement data validation
- Add comprehensive tests

## Running Tests

```bash
cd src/tests
python -m pytest
```
EOF

# 4. Initial commit
git add .
git commit -m "Initial commit: sample Python project with intentional issues" --quiet
echo "Created sample codebase with TODOs and bugs"

# 5. Initialize grit
echo ""
echo "Initializing grit..."
if command -v grit &> /dev/null; then
    grit init 2>/dev/null || echo "  (grit already initialized or not available)"
else
    echo "  WARNING: grit not found in PATH"
fi

# 6. Initialize brat (no daemon, no tmux for testing)
echo "Initializing brat..."
if command -v brat &> /dev/null; then
    brat init --no-daemon --no-tmux 2>/dev/null || echo "  (brat already initialized)"
else
    echo "  WARNING: brat not found in PATH"
    echo "  Make sure to build brat: cargo build --release -p brat"
    echo "  And add to PATH: export PATH=\"\$PATH:$BRAT_REPO/target/release\""
fi

# 7. Create workflows directory and copy templates
echo "Setting up workflow templates..."
mkdir -p "$TEST_DIR/.brat/workflows"

if [ -d "$BRAT_REPO/.brat/workflows" ]; then
    cp -r "$BRAT_REPO/.brat/workflows"/* "$TEST_DIR/.brat/workflows/" 2>/dev/null || true
    echo "  Copied workflow templates from $BRAT_REPO"
else
    # Create basic workflow templates if source not found
    cat > "$TEST_DIR/.brat/workflows/fix-bug.yaml" << 'EOF'
name: fix-bug
version: 1
description: "Fix a bug in the codebase"
type: workflow

inputs:
  bug:
    description: "Description of the bug"
    required: true

steps:
  - id: investigate
    title: "Investigate {{bug}}"
    body: |
      Investigate the bug: {{bug}}

      Steps:
      1. Identify the root cause
      2. Find affected code
      3. Document findings

  - id: fix
    title: "Fix {{bug}}"
    needs: [investigate]
    body: |
      Fix the bug based on investigation.

      Guidelines:
      - Write minimal, focused fix
      - Don't refactor unrelated code
      - Add comments explaining the fix

  - id: test
    title: "Test fix for {{bug}}"
    needs: [fix]
    body: |
      Add tests to verify the fix and prevent regression.

      Required:
      - Test that reproduces the original bug
      - Test that verifies the fix works
EOF
    echo "  Created default workflow templates"
fi

# 8. Commit brat config
git add .gitignore .brat 2>/dev/null || true
git commit -m "Initialize brat harness" --quiet --allow-empty

echo ""
echo "=============================================="
echo "  SETUP COMPLETE"
echo "=============================================="
echo ""
echo "Test repository created at: $TEST_DIR"
echo ""
echo "Sample codebase includes:"
echo "  - src/main.py    (2 TODOs)"
echo "  - src/utils.py   (2 bugs, 3 TODOs)"
echo "  - src/config.py  (3 TODOs, 1 bug)"
echo "  - src/tests/     (incomplete tests)"
echo ""
echo "Next steps:"
echo ""
echo "  1. Navigate to the test repo:"
echo "     cd $TEST_DIR"
echo ""
echo "  2. Start the Mayor:"
echo "     brat mayor start"
echo ""
echo "  3. Ask Mayor to analyze and create tasks:"
echo "     brat mayor ask 'Analyze this codebase and create a convoy to fix all bugs'"
echo ""
echo "  4. Check status:"
echo "     brat status"
echo ""
echo "  5. View Mayor conversation:"
echo "     brat mayor tail"
echo ""
echo "  6. Stop Mayor when done:"
echo "     brat mayor stop"
echo ""

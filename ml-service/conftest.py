"""Pytest configuration for ML service tests.

This file configures pytest to find the modules in the math/ directory.
"""

import sys
from pathlib import Path

# Add the math directory to Python path so tests can import from it
math_dir = Path(__file__).parent.parent / "math"
if str(math_dir) not in sys.path:
    sys.path.insert(0, str(math_dir))

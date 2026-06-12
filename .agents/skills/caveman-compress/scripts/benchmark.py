#!/usr/bin/env python3
from pathlib import Path
import sys

# Support both direct execution and module import
try:
    from .validate import validate
except ImportError:
    sys.path.insert(0, str(Path(__file__).parent))
    from validate import validate

try:
    import tiktoken
    _enc = tiktoken.get_encoding("o200k_base")
except ImportError:
    _enc = None


def count_tokens(text):
    if _enc is None:
        return len(text.split())  # fallback: word count
    return len(_enc.encode(text))


def benchmark_pair(orig_path: Path, comp_path: Path):
    orig_text = orig_path.read_text()

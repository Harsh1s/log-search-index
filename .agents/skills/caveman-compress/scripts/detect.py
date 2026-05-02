#!/usr/bin/env python3
"""Detect whether a file is natural language (compressible) or code/config (skip)."""

import json
import re
from pathlib import Path

# Extensions that are natural language and compressible
COMPRESSIBLE_EXTENSIONS = {".md", ".txt", ".markdown", ".rst", ".typ", ".typst", ".tex"}

# Extensions that are code/config and should be skipped
SKIP_EXTENSIONS = {
    ".py", ".js", ".ts", ".tsx", ".jsx", ".json", ".yaml", ".yml",
    ".toml", ".env", ".lock", ".css", ".scss", ".html", ".xml",
    ".sql", ".sh", ".bash", ".zsh", ".go", ".rs", ".java", ".c",
    ".cpp", ".h", ".hpp", ".rb", ".php", ".swift", ".kt", ".lua",
    ".dockerfile", ".makefile", ".csv", ".ini", ".cfg",
}

# Patterns that indicate a line is code
CODE_PATTERNS = [
    re.compile(r"^\s*(import |from .+ import |require\(|const |let |var )"),
    re.compile(r"^\s*(def |class |function |async function |export )"),
    re.compile(r"^\s*(if\s*\(|for\s*\(|while\s*\(|switch\s*\(|try\s*\{)"),
    re.compile(r"^\s*[\}\]\);]+\s*$"),  # closing braces/brackets
    re.compile(r"^\s*@\w+"),  # decorators/annotations
    re.compile(r'^\s*"[^"]+"\s*:\s*'),  # JSON-like key-value
    re.compile(r"^\s*\w+\s*=\s*[{\[\(\"']"),  # assignment with literal
]


def _is_code_line(line: str) -> bool:
    """Check if a line looks like code."""
    return any(p.match(line) for p in CODE_PATTERNS)


def _is_json_content(text: str) -> bool:
    """Check if content is valid JSON."""
    try:
        json.loads(text)

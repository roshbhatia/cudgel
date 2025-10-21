"""Tests for the code parser."""

import pytest
from pathlib import Path
from cudgel.parser import CodeParser


def test_detect_language():
    """Test language detection."""
    parser = CodeParser()

    assert parser.detect_language(Path("test.py")) == "python"
    assert parser.detect_language(Path("test.js")) == "javascript"
    assert parser.detect_language(Path("test.ts")) == "typescript"
    assert parser.detect_language(Path("test.rs")) == "rust"
    assert parser.detect_language(Path("test.go")) == "go"
    assert parser.detect_language(Path("test.unknown")) is None


def test_parse_python_code():
    """Test parsing Python code."""
    parser = CodeParser()

    code = '''
def hello(name):
    """Say hello."""
    return f"Hello, {name}!"

class Greeter:
    def greet(self):
        return "Hello"
'''

    # Create a temporary file
    import tempfile
    with tempfile.NamedTemporaryFile(mode='w', suffix='.py', delete=False) as f:
        f.write(code)
        temp_path = Path(f.name)

    try:
        ast_root, content, file_hash = parser.parse_file(temp_path)

        assert ast_root is not None
        assert ast_root.node_type == "module"
        assert len(ast_root.children) > 0
        assert content == code
        assert len(file_hash) == 64  # SHA-256 hex digest

        # Extract symbols
        symbols = parser.extract_symbols(ast_root, "python")
        assert len(symbols) > 0

        # Check for function and class
        symbol_names = {s.name for s in symbols}
        assert "hello" in symbol_names or "Greeter" in symbol_names

    finally:
        temp_path.unlink()


def test_parse_javascript_code():
    """Test parsing JavaScript code."""
    parser = CodeParser()

    code = '''
function add(a, b) {
    return a + b;
}

class Calculator {
    multiply(a, b) {
        return a * b;
    }
}
'''

    import tempfile
    with tempfile.NamedTemporaryFile(mode='w', suffix='.js', delete=False) as f:
        f.write(code)
        temp_path = Path(f.name)

    try:
        ast_root, content, file_hash = parser.parse_file(temp_path)

        assert ast_root is not None
        assert content == code

        symbols = parser.extract_symbols(ast_root, "javascript")
        assert len(symbols) > 0

    finally:
        temp_path.unlink()

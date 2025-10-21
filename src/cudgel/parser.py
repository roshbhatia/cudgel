"""Tree-sitter based code parsing."""

import hashlib
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Optional

import tree_sitter_languages


@dataclass
class ASTNode:
    """Represents an AST node."""

    node_type: str
    text: str
    start_line: int
    start_column: int
    end_line: int
    end_column: int
    children: list["ASTNode"]
    parent: Optional["ASTNode"] = None


@dataclass
class Symbol:
    """Represents a code symbol (function, class, variable, etc.)."""

    name: str
    kind: str  # function, class, method, variable, etc.
    signature: Optional[str]
    docstring: Optional[str]
    start_line: int
    end_line: int
    text: str
    ast_node: ASTNode


class CodeParser:
    """Tree-sitter based code parser."""

    # Language file extensions mapping
    LANGUAGE_EXTENSIONS = {
        "python": [".py", ".pyw"],
        "javascript": [".js", ".jsx", ".mjs"],
        "typescript": [".ts", ".tsx"],
        "rust": [".rs"],
        "go": [".go"],
        "c": [".c", ".h"],
        "cpp": [".cpp", ".cc", ".cxx", ".hpp", ".hh", ".hxx"],
        "java": [".java"],
        "c_sharp": [".cs"],
        "ruby": [".rb"],
        "php": [".php"],
        "swift": [".swift"],
        "kotlin": [".kt", ".kts"],
    }

    # Symbol node types for different languages
    SYMBOL_QUERIES = {
        "python": {
            "function": ["function_definition"],
            "class": ["class_definition"],
            "variable": ["assignment"],
            "import": ["import_statement", "import_from_statement"],
        },
        "javascript": {
            "function": ["function_declaration", "arrow_function", "function_expression"],
            "class": ["class_declaration"],
            "variable": ["variable_declarator"],
            "import": ["import_statement"],
        },
        "typescript": {
            "function": ["function_declaration", "arrow_function", "method_definition"],
            "class": ["class_declaration", "interface_declaration"],
            "variable": ["variable_declarator"],
            "import": ["import_statement"],
        },
        "rust": {
            "function": ["function_item"],
            "class": ["struct_item", "enum_item", "trait_item"],
            "variable": ["let_declaration"],
            "import": ["use_declaration"],
        },
        "go": {
            "function": ["function_declaration", "method_declaration"],
            "class": ["type_declaration"],
            "variable": ["var_declaration", "short_var_declaration"],
            "import": ["import_declaration"],
        },
    }

    def __init__(self):
        self.parsers: dict[str, Any] = {}

    def detect_language(self, file_path: Path) -> Optional[str]:
        """Detect programming language from file extension."""
        ext = file_path.suffix.lower()
        for lang, extensions in self.LANGUAGE_EXTENSIONS.items():
            if ext in extensions:
                return lang
        return None

    def get_parser(self, language: str) -> Any:
        """Get or create a tree-sitter parser for the language."""
        if language not in self.parsers:
            try:
                self.parsers[language] = tree_sitter_languages.get_parser(language)
            except Exception as e:
                raise ValueError(f"Unsupported language: {language}") from e
        return self.parsers[language]

    def parse_file(self, file_path: Path, content: Optional[str] = None) -> tuple[ASTNode, str, str]:
        """
        Parse a source file and return the AST.

        Returns:
            Tuple of (root_ast_node, content, file_hash)
        """
        language = self.detect_language(file_path)
        if not language:
            raise ValueError(f"Cannot detect language for file: {file_path}")

        if content is None:
            content = file_path.read_text(encoding="utf-8", errors="ignore")

        # Calculate file hash
        file_hash = hashlib.sha256(content.encode()).hexdigest()

        parser = self.get_parser(language)
        tree = parser.parse(bytes(content, "utf8"))

        root_node = self._convert_tree_sitter_node(tree.root_node, content)
        return root_node, content, file_hash

    def _convert_tree_sitter_node(
        self,
        ts_node: Any,
        content: str,
        parent: Optional[ASTNode] = None
    ) -> ASTNode:
        """Convert tree-sitter node to our ASTNode."""
        node = ASTNode(
            node_type=ts_node.type,
            text=content[ts_node.start_byte:ts_node.end_byte],
            start_line=ts_node.start_point[0],
            start_column=ts_node.start_point[1],
            end_line=ts_node.end_point[0],
            end_column=ts_node.end_point[1],
            children=[],
            parent=parent,
        )

        for child in ts_node.children:
            child_node = self._convert_tree_sitter_node(child, content, node)
            node.children.append(child_node)

        return node

    def extract_symbols(self, ast_node: ASTNode, language: str) -> list[Symbol]:
        """Extract symbols (functions, classes, etc.) from AST."""
        symbols: list[Symbol] = []
        symbol_types = self.SYMBOL_QUERIES.get(language, {})

        self._extract_symbols_recursive(ast_node, language, symbol_types, symbols)
        return symbols

    def _extract_symbols_recursive(
        self,
        node: ASTNode,
        language: str,
        symbol_types: dict[str, list[str]],
        symbols: list[Symbol],
    ) -> None:
        """Recursively extract symbols from AST."""
        for kind, node_types in symbol_types.items():
            if node.node_type in node_types:
                symbol = self._create_symbol(node, kind, language)
                if symbol:
                    symbols.append(symbol)

        for child in node.children:
            self._extract_symbols_recursive(child, language, symbol_types, symbols)

    def _create_symbol(self, node: ASTNode, kind: str, language: str) -> Optional[Symbol]:
        """Create a symbol from an AST node."""
        name = self._extract_symbol_name(node, language)
        if not name:
            return None

        signature = self._extract_signature(node, language)
        docstring = self._extract_docstring(node, language)

        return Symbol(
            name=name,
            kind=kind,
            signature=signature,
            docstring=docstring,
            start_line=node.start_line,
            end_line=node.end_line,
            text=node.text,
            ast_node=node,
        )

    def _extract_symbol_name(self, node: ASTNode, language: str) -> Optional[str]:
        """Extract symbol name from node."""
        # Look for identifier child nodes
        for child in node.children:
            if "identifier" in child.node_type.lower() or "name" in child.node_type.lower():
                return child.text.strip()
        return None

    def _extract_signature(self, node: ASTNode, language: str) -> Optional[str]:
        """Extract function/method signature."""
        if node.node_type in ["function_definition", "function_declaration", "method_definition"]:
            # Get the first line as signature
            lines = node.text.split('\n')
            if lines:
                return lines[0].strip()
        return None

    def _extract_docstring(self, node: ASTNode, language: str) -> Optional[str]:
        """Extract docstring/documentation."""
        if language == "python":
            # Look for string literal as first child of function body
            for child in node.children:
                if child.node_type == "block":
                    for stmt in child.children:
                        if "string" in stmt.node_type.lower():
                            return stmt.text.strip().strip('"""').strip("'''").strip()
        return None

    def extract_references(self, ast_node: ASTNode, symbols: list[Symbol], language: str) -> list[dict[str, Any]]:
        """Extract references between symbols."""
        references: list[dict[str, Any]] = []
        symbol_names = {s.name for s in symbols}

        self._extract_references_recursive(ast_node, symbol_names, references, language)
        return references

    def _extract_references_recursive(
        self,
        node: ASTNode,
        symbol_names: set[str],
        references: list[dict[str, Any]],
        language: str,
    ) -> None:
        """Recursively extract references from AST."""
        # Look for identifier nodes that match known symbols
        if "identifier" in node.node_type.lower() or "name" in node.node_type.lower():
            if node.text in symbol_names:
                references.append({
                    "name": node.text,
                    "line": node.start_line,
                    "column": node.start_column,
                    "type": "reference",
                })

        for child in node.children:
            self._extract_references_recursive(child, symbol_names, references, language)

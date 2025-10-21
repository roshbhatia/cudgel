"""Language Server Protocol (LSP) server for cudgel."""

import asyncio
from typing import Optional

from lsprotocol.types import (
    TEXT_DOCUMENT_COMPLETION,
    TEXT_DOCUMENT_HOVER,
    CompletionItem,
    CompletionList,
    CompletionParams,
    Hover,
    HoverParams,
    MarkupContent,
    MarkupKind,
)
from pygls.server import LanguageServer

from cudgel.config import get_config
from cudgel.query import CodeQuery


class CudgelLSPServer(LanguageServer):
    """LSP server for code intelligence powered by cudgel."""

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.config = get_config()
        self.query_engine: Optional[CodeQuery] = None

    async def initialize_query_engine(self) -> None:
        """Initialize the query engine."""
        if self.query_engine is None:
            self.query_engine = CodeQuery(self.config)
            await self.query_engine.initialize()


server = CudgelLSPServer("cudgel-lsp", "v0.1")


@server.feature(TEXT_DOCUMENT_COMPLETION)
async def completions(params: CompletionParams) -> CompletionList:
    """Provide code completions based on indexed code."""
    await server.initialize_query_engine()

    # Get current word/context
    document = server.workspace.get_document(params.text_document.uri)
    current_line = document.lines[params.position.line]
    current_word = _get_word_at_position(current_line, params.position.character)

    if not current_word or len(current_word) < 2:
        return CompletionList(is_incomplete=False, items=[])

    # Query for similar symbols
    if server.query_engine:
        results = await server.query_engine.search_symbols(
            current_word,
            limit=20,
        )

        items = []
        for result in results:
            item = CompletionItem(
                label=result["name"],
                kind=_symbol_kind_to_completion_kind(result["kind"]),
                detail=f"{result['kind']} - {result['path']}",
                documentation=result.get("docstring"),
            )
            items.append(item)

        return CompletionList(is_incomplete=False, items=items)

    return CompletionList(is_incomplete=False, items=[])


@server.feature(TEXT_DOCUMENT_HOVER)
async def hover(params: HoverParams) -> Optional[Hover]:
    """Provide hover information for symbols."""
    await server.initialize_query_engine()

    # Get word at position
    document = server.workspace.get_document(params.text_document.uri)
    current_line = document.lines[params.position.line]
    current_word = _get_word_at_position(current_line, params.position.character)

    if not current_word or not server.query_engine:
        return None

    # Search for exact symbol match
    results = await server.query_engine.search_symbols(
        current_word,
        limit=1,
    )

    if results:
        result = results[0]
        content_parts = [f"**{result['name']}** ({result['kind']})"]

        if result.get("signature"):
            content_parts.append(f"\n```{result['language']}\n{result['signature']}\n```")

        if result.get("docstring"):
            content_parts.append(f"\n{result['docstring']}")

        content_parts.append(f"\n\n*Defined in: {result['path']}:{result['start_line']}*")

        return Hover(
            contents=MarkupContent(
                kind=MarkupKind.Markdown,
                value="\n".join(content_parts),
            )
        )

    return None


def _get_word_at_position(line: str, character: int) -> str:
    """Extract word at cursor position."""
    if character >= len(line):
        character = len(line) - 1

    # Find word boundaries
    start = character
    while start > 0 and (line[start - 1].isalnum() or line[start - 1] == "_"):
        start -= 1

    end = character
    while end < len(line) and (line[end].isalnum() or line[end] == "_"):
        end += 1

    return line[start:end]


def _symbol_kind_to_completion_kind(symbol_kind: str) -> int:
    """Map symbol kind to LSP completion kind."""
    mapping = {
        "function": 3,  # Function
        "method": 2,    # Method
        "class": 7,     # Class
        "variable": 6,  # Variable
        "import": 9,    # Module
    }
    return mapping.get(symbol_kind, 1)  # Default to Text


def start_lsp_server(host: str = "localhost", port: int = 6010) -> None:
    """Start the LSP server."""
    server.start_tcp(host, port)

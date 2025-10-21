"""Graph-based relationship queries."""

from typing import Any, Optional

from cudgel.config import CudgelConfig
from cudgel.database import Database


class GraphQuery:
    """Query code relationships as a graph."""

    def __init__(self, config: CudgelConfig):
        self.config = config
        self.db = Database(config)

    async def initialize(self) -> None:
        """Initialize the graph query system."""
        await self.db.connect()

    async def close(self) -> None:
        """Close connections."""
        await self.db.close()

    async def get_symbol_by_name(
        self,
        name: str,
        repository_path: Optional[str] = None
    ) -> Optional[dict[str, Any]]:
        """Get a symbol by name."""
        if not self.db.conn:
            await self.db.connect()

        async with self.db.conn.cursor() as cur:
            if repository_path:
                await cur.execute(
                    """
                    SELECT s.*, f.path, f.language, r.path as repo_path
                    FROM symbols s
                    JOIN files f ON s.file_id = f.id
                    JOIN repositories r ON f.repository_id = r.id
                    WHERE s.name = %s AND r.path = %s
                    LIMIT 1
                    """,
                    (name, repository_path),
                )
            else:
                await cur.execute(
                    """
                    SELECT s.*, f.path, f.language, r.path as repo_path
                    FROM symbols s
                    JOIN files f ON s.file_id = f.id
                    JOIN repositories r ON f.repository_id = r.id
                    WHERE s.name = %s
                    LIMIT 1
                    """,
                    (name,),
                )

            return await cur.fetchone()

    async def get_references(
        self,
        symbol_name: str,
        repository_path: Optional[str] = None,
        depth: int = 1
    ) -> dict[str, Any]:
        """
        Get reference graph for a symbol.

        Args:
            symbol_name: Name of the symbol
            repository_path: Optional filter by repository
            depth: How many levels deep to traverse

        Returns:
            Graph structure with nodes and edges
        """
        symbol = await self.get_symbol_by_name(symbol_name, repository_path)
        if not symbol:
            return {"nodes": [], "edges": [], "error": "Symbol not found"}

        nodes: dict[int, dict[str, Any]] = {}
        edges: list[dict[str, Any]] = []

        await self._traverse_references(symbol["id"], nodes, edges, depth, set())

        return {
            "nodes": list(nodes.values()),
            "edges": edges,
            "root": symbol_name,
        }

    async def _traverse_references(
        self,
        symbol_id: int,
        nodes: dict[int, dict[str, Any]],
        edges: list[dict[str, Any]],
        depth: int,
        visited: set[int],
    ) -> None:
        """Recursively traverse references."""
        if depth <= 0 or symbol_id in visited:
            return

        visited.add(symbol_id)

        if not self.db.conn:
            return

        # Get symbol info
        async with self.db.conn.cursor() as cur:
            await cur.execute(
                """
                SELECT s.*, f.path, f.language
                FROM symbols s
                JOIN files f ON s.file_id = f.id
                WHERE s.id = %s
                """,
                (symbol_id,),
            )
            symbol = await cur.fetchone()

            if symbol:
                nodes[symbol_id] = {
                    "id": symbol["id"],
                    "name": symbol["name"],
                    "kind": symbol["kind"],
                    "file": symbol["path"],
                    "language": symbol["language"],
                    "line": symbol["start_line"],
                }

            # Get outgoing references
            await cur.execute(
                """
                SELECT
                    r.*,
                    s.name as to_name,
                    s.kind as to_kind
                FROM references r
                JOIN symbols s ON r.to_symbol_id = s.id
                WHERE r.from_symbol_id = %s
                """,
                (symbol_id,),
            )
            references = await cur.fetchall()

            for ref in references:
                edges.append({
                    "from": symbol_id,
                    "to": ref["to_symbol_id"],
                    "type": ref["reference_type"],
                    "line": ref["line"],
                    "column": ref["column"],
                })

                # Recursively traverse
                await self._traverse_references(
                    ref["to_symbol_id"],
                    nodes,
                    edges,
                    depth - 1,
                    visited,
                )

    async def get_call_graph(
        self,
        symbol_name: str,
        repository_path: Optional[str] = None,
        direction: str = "outgoing"
    ) -> dict[str, Any]:
        """
        Get call graph for a function/method.

        Args:
            symbol_name: Name of the function/method
            repository_path: Optional filter by repository
            direction: "outgoing" (what this calls), "incoming" (what calls this), or "both"

        Returns:
            Call graph structure
        """
        symbol = await self.get_symbol_by_name(symbol_name, repository_path)
        if not symbol:
            return {"nodes": [], "edges": [], "error": "Symbol not found"}

        if symbol["kind"] not in ("function", "method"):
            return {"nodes": [], "edges": [], "error": "Symbol is not a function or method"}

        nodes: dict[int, dict[str, Any]] = {}
        edges: list[dict[str, Any]] = []

        if not self.db.conn:
            await self.db.connect()

        # Add root node
        nodes[symbol["id"]] = {
            "id": symbol["id"],
            "name": symbol["name"],
            "kind": symbol["kind"],
            "file": symbol["path"],
            "language": symbol["language"],
            "line": symbol["start_line"],
        }

        async with self.db.conn.cursor() as cur:
            if direction in ("outgoing", "both"):
                # Get functions this one calls
                await cur.execute(
                    """
                    SELECT
                        r.*,
                        s.id as to_id,
                        s.name as to_name,
                        s.kind as to_kind,
                        f.path as to_file
                    FROM references r
                    JOIN symbols s ON r.to_symbol_id = s.id
                    JOIN files f ON s.file_id = f.id
                    WHERE r.from_symbol_id = %s
                    AND s.kind IN ('function', 'method')
                    """,
                    (symbol["id"],),
                )
                outgoing = await cur.fetchall()

                for ref in outgoing:
                    nodes[ref["to_id"]] = {
                        "id": ref["to_id"],
                        "name": ref["to_name"],
                        "kind": ref["to_kind"],
                        "file": ref["to_file"],
                    }
                    edges.append({
                        "from": symbol["id"],
                        "to": ref["to_id"],
                        "type": "calls",
                    })

            if direction in ("incoming", "both"):
                # Get functions that call this one
                await cur.execute(
                    """
                    SELECT
                        r.*,
                        s.id as from_id,
                        s.name as from_name,
                        s.kind as from_kind,
                        f.path as from_file
                    FROM references r
                    JOIN symbols s ON r.from_symbol_id = s.id
                    JOIN files f ON s.file_id = f.id
                    WHERE r.to_symbol_id = %s
                    AND s.kind IN ('function', 'method')
                    """,
                    (symbol["id"],),
                )
                incoming = await cur.fetchall()

                for ref in incoming:
                    nodes[ref["from_id"]] = {
                        "id": ref["from_id"],
                        "name": ref["from_name"],
                        "kind": ref["from_kind"],
                        "file": ref["from_file"],
                    }
                    edges.append({
                        "from": ref["from_id"],
                        "to": symbol["id"],
                        "type": "calls",
                    })

        return {
            "nodes": list(nodes.values()),
            "edges": edges,
            "root": symbol_name,
            "direction": direction,
        }

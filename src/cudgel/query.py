"""Natural language query functionality."""

from typing import Any, Optional

from cudgel.config import CudgelConfig
from cudgel.database import Database
from cudgel.embeddings import EmbeddingGenerator


class CodeQuery:
    """Query code using natural language."""

    def __init__(self, config: CudgelConfig):
        self.config = config
        self.db = Database(config)
        self.embedder = EmbeddingGenerator(config)

    async def initialize(self) -> None:
        """Initialize the query system."""
        await self.db.connect()
        self.embedder.load_model()

    async def close(self) -> None:
        """Close connections."""
        await self.db.close()

    async def search_symbols(
        self,
        query: str,
        limit: int = 10,
        repository_path: Optional[str] = None
    ) -> list[dict[str, Any]]:
        """
        Search for symbols using natural language query.

        Args:
            query: Natural language query
            limit: Maximum number of results
            repository_path: Optional filter by repository path

        Returns:
            List of matching symbols with metadata
        """
        # Generate query embedding
        query_embedding = self.embedder.encode_query(query)

        if not self.db.conn:
            await self.db.connect()

        # Search for similar symbols using vector similarity
        async with self.db.conn.cursor() as cur:
            if repository_path:
                await cur.execute(
                    """
                    SELECT
                        s.id,
                        s.name,
                        s.kind,
                        s.signature,
                        s.docstring,
                        s.start_line,
                        s.end_line,
                        f.path,
                        f.language,
                        r.path as repo_path,
                        r.name as repo_name,
                        1 - (s.embedding <=> %s::vector) as similarity
                    FROM symbols s
                    JOIN files f ON s.file_id = f.id
                    JOIN repositories r ON f.repository_id = r.id
                    WHERE r.path = %s
                    ORDER BY s.embedding <=> %s::vector
                    LIMIT %s
                    """,
                    (query_embedding.tolist(), repository_path, query_embedding.tolist(), limit),
                )
            else:
                await cur.execute(
                    """
                    SELECT
                        s.id,
                        s.name,
                        s.kind,
                        s.signature,
                        s.docstring,
                        s.start_line,
                        s.end_line,
                        f.path,
                        f.language,
                        r.path as repo_path,
                        r.name as repo_name,
                        1 - (s.embedding <=> %s::vector) as similarity
                    FROM symbols s
                    JOIN files f ON s.file_id = f.id
                    JOIN repositories r ON f.repository_id = r.id
                    ORDER BY s.embedding <=> %s::vector
                    LIMIT %s
                    """,
                    (query_embedding.tolist(), query_embedding.tolist(), limit),
                )

            results = await cur.fetchall()
            return list(results)

    async def search_code(
        self,
        query: str,
        limit: int = 10,
        repository_path: Optional[str] = None
    ) -> list[dict[str, Any]]:
        """
        Search for code chunks using natural language query.

        Args:
            query: Natural language query
            limit: Maximum number of results
            repository_path: Optional filter by repository path

        Returns:
            List of matching code chunks with metadata
        """
        # Generate query embedding
        query_embedding = self.embedder.encode_query(query)

        if not self.db.conn:
            await self.db.connect()

        # Search for similar code chunks
        async with self.db.conn.cursor() as cur:
            if repository_path:
                await cur.execute(
                    """
                    SELECT
                        c.id,
                        c.text,
                        c.start_line,
                        c.end_line,
                        s.name as symbol_name,
                        s.kind as symbol_kind,
                        f.path,
                        f.language,
                        r.path as repo_path,
                        r.name as repo_name,
                        1 - (c.embedding <=> %s::vector) as similarity
                    FROM code_chunks c
                    JOIN files f ON c.file_id = f.id
                    JOIN repositories r ON f.repository_id = r.id
                    LEFT JOIN symbols s ON c.symbol_id = s.id
                    WHERE r.path = %s
                    ORDER BY c.embedding <=> %s::vector
                    LIMIT %s
                    """,
                    (query_embedding.tolist(), repository_path, query_embedding.tolist(), limit),
                )
            else:
                await cur.execute(
                    """
                    SELECT
                        c.id,
                        c.text,
                        c.start_line,
                        c.end_line,
                        s.name as symbol_name,
                        s.kind as symbol_kind,
                        f.path,
                        f.language,
                        r.path as repo_path,
                        r.name as repo_name,
                        1 - (c.embedding <=> %s::vector) as similarity
                    FROM code_chunks c
                    JOIN files f ON c.file_id = f.id
                    JOIN repositories r ON f.repository_id = r.id
                    LEFT JOIN symbols s ON c.symbol_id = s.id
                    ORDER BY c.embedding <=> %s::vector
                    LIMIT %s
                    """,
                    (query_embedding.tolist(), query_embedding.tolist(), limit),
                )

            results = await cur.fetchall()
            return list(results)

    async def search(
        self,
        query: str,
        limit: int = 10,
        repository_path: Optional[str] = None,
        search_type: str = "both"
    ) -> dict[str, Any]:
        """
        Unified search combining symbols and code chunks.

        Args:
            query: Natural language query
            limit: Maximum number of results per type
            repository_path: Optional filter by repository path
            search_type: "symbols", "code", or "both"

        Returns:
            Dictionary with symbols and/or code results
        """
        results: dict[str, Any] = {}

        if search_type in ("symbols", "both"):
            results["symbols"] = await self.search_symbols(query, limit, repository_path)

        if search_type in ("code", "both"):
            results["code"] = await self.search_code(query, limit, repository_path)

        return results

"""Database models and connection management."""

import json
from datetime import datetime
from typing import Any, List, Optional

import psycopg
from pgvector.psycopg import register_vector
from psycopg.rows import dict_row

from cudgel.config import CudgelConfig


class Database:
    """PostgreSQL database connection and operations."""

    def __init__(self, config: CudgelConfig):
        self.config = config
        self.conn: Optional[psycopg.Connection] = None

    async def connect(self) -> None:
        """Establish database connection."""
        self.conn = await psycopg.AsyncConnection.connect(
            self.config.database_url,
            row_factory=dict_row,
        )
        await register_vector(self.conn)

    async def close(self) -> None:
        """Close database connection."""
        if self.conn:
            await self.conn.close()

    async def init_schema(self) -> None:
        """Initialize database schema with pgvector extension."""
        if not self.conn:
            await self.connect()

        async with self.conn.cursor() as cur:
            # Enable pgvector extension
            await cur.execute("CREATE EXTENSION IF NOT EXISTS vector")

            # Repositories table
            await cur.execute("""
                CREATE TABLE IF NOT EXISTS repositories (
                    id SERIAL PRIMARY KEY,
                    path TEXT NOT NULL UNIQUE,
                    name TEXT NOT NULL,
                    indexed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    last_updated TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    metadata JSONB DEFAULT '{}'
                )
            """)

            # Files table
            await cur.execute("""
                CREATE TABLE IF NOT EXISTS files (
                    id SERIAL PRIMARY KEY,
                    repository_id INTEGER REFERENCES repositories(id) ON DELETE CASCADE,
                    path TEXT NOT NULL,
                    language TEXT,
                    content TEXT,
                    hash TEXT NOT NULL,
                    indexed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    metadata JSONB DEFAULT '{}',
                    UNIQUE(repository_id, path)
                )
            """)

            # AST nodes table - tree structure
            await cur.execute("""
                CREATE TABLE IF NOT EXISTS ast_nodes (
                    id SERIAL PRIMARY KEY,
                    file_id INTEGER REFERENCES files(id) ON DELETE CASCADE,
                    parent_id INTEGER REFERENCES ast_nodes(id) ON DELETE CASCADE,
                    node_type TEXT NOT NULL,
                    text TEXT,
                    start_line INTEGER NOT NULL,
                    start_column INTEGER NOT NULL,
                    end_line INTEGER NOT NULL,
                    end_column INTEGER NOT NULL,
                    metadata JSONB DEFAULT '{}'
                )
            """)

            # Create index on parent_id for efficient tree traversal
            await cur.execute("""
                CREATE INDEX IF NOT EXISTS idx_ast_nodes_parent
                ON ast_nodes(parent_id)
            """)

            await cur.execute("""
                CREATE INDEX IF NOT EXISTS idx_ast_nodes_file
                ON ast_nodes(file_id)
            """)

            # Symbols table - functions, classes, variables
            await cur.execute(f"""
                CREATE TABLE IF NOT EXISTS symbols (
                    id SERIAL PRIMARY KEY,
                    file_id INTEGER REFERENCES files(id) ON DELETE CASCADE,
                    ast_node_id INTEGER REFERENCES ast_nodes(id) ON DELETE CASCADE,
                    name TEXT NOT NULL,
                    kind TEXT NOT NULL,
                    signature TEXT,
                    docstring TEXT,
                    start_line INTEGER NOT NULL,
                    end_line INTEGER NOT NULL,
                    embedding vector({self.config.embedding_dimension}),
                    metadata JSONB DEFAULT '{{}}'
                )
            """)

            # Create vector index for similarity search
            await cur.execute("""
                CREATE INDEX IF NOT EXISTS idx_symbols_embedding
                ON symbols USING ivfflat (embedding vector_cosine_ops)
                WITH (lists = 100)
            """)

            # References table - for graph relationships
            await cur.execute("""
                CREATE TABLE IF NOT EXISTS references (
                    id SERIAL PRIMARY KEY,
                    from_symbol_id INTEGER REFERENCES symbols(id) ON DELETE CASCADE,
                    to_symbol_id INTEGER REFERENCES symbols(id) ON DELETE CASCADE,
                    reference_type TEXT NOT NULL,
                    file_id INTEGER REFERENCES files(id) ON DELETE CASCADE,
                    line INTEGER NOT NULL,
                    column INTEGER NOT NULL,
                    metadata JSONB DEFAULT '{}',
                    UNIQUE(from_symbol_id, to_symbol_id, reference_type, line, column)
                )
            """)

            # Create indexes for graph queries
            await cur.execute("""
                CREATE INDEX IF NOT EXISTS idx_references_from
                ON references(from_symbol_id)
            """)

            await cur.execute("""
                CREATE INDEX IF NOT EXISTS idx_references_to
                ON references(to_symbol_id)
            """)

            # Code chunks table - for semantic search
            await cur.execute(f"""
                CREATE TABLE IF NOT EXISTS code_chunks (
                    id SERIAL PRIMARY KEY,
                    file_id INTEGER REFERENCES files(id) ON DELETE CASCADE,
                    symbol_id INTEGER REFERENCES symbols(id) ON DELETE SET NULL,
                    text TEXT NOT NULL,
                    start_line INTEGER NOT NULL,
                    end_line INTEGER NOT NULL,
                    embedding vector({self.config.embedding_dimension}),
                    metadata JSONB DEFAULT '{{}}'
                )
            """)

            # Create vector index for code chunks
            await cur.execute("""
                CREATE INDEX IF NOT EXISTS idx_code_chunks_embedding
                ON code_chunks USING ivfflat (embedding vector_cosine_ops)
                WITH (lists = 100)
            """)

            await self.conn.commit()

    async def add_repository(self, path: str, name: str, metadata: dict[str, Any] | None = None) -> int:
        """Add a repository to the database."""
        if not self.conn:
            await self.connect()

        async with self.conn.cursor() as cur:
            await cur.execute(
                """
                INSERT INTO repositories (path, name, metadata)
                VALUES (%s, %s, %s)
                ON CONFLICT (path) DO UPDATE
                SET last_updated = CURRENT_TIMESTAMP
                RETURNING id
                """,
                (path, name, json.dumps(metadata or {})),
            )
            result = await cur.fetchone()
            await self.conn.commit()
            return result["id"] if result else -1

    async def get_repository(self, path: str) -> Optional[dict[str, Any]]:
        """Get repository by path."""
        if not self.conn:
            await self.connect()

        async with self.conn.cursor() as cur:
            await cur.execute(
                "SELECT * FROM repositories WHERE path = %s",
                (path,),
            )
            return await cur.fetchone()

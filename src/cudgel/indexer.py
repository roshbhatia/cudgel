"""Core indexing functionality."""

import json
from pathlib import Path
from typing import Any, Optional

import psycopg
from pgvector.psycopg import register_vector

from cudgel.config import CudgelConfig
from cudgel.database import Database
from cudgel.embeddings import EmbeddingGenerator
from cudgel.parser import ASTNode, CodeParser, Symbol


class CodeIndexer:
    """Index code repositories using tree-sitter and embeddings."""

    def __init__(self, config: CudgelConfig):
        self.config = config
        self.db = Database(config)
        self.parser = CodeParser()
        self.embedder = EmbeddingGenerator(config)

    async def initialize(self) -> None:
        """Initialize the indexer."""
        await self.db.connect()
        await self.db.init_schema()
        self.embedder.load_model()

    async def close(self) -> None:
        """Close connections."""
        await self.db.close()

    async def index_repository(self, repo_path: Path, name: Optional[str] = None) -> int:
        """
        Index a code repository.

        Args:
            repo_path: Path to the repository
            name: Optional repository name (defaults to directory name)

        Returns:
            Repository ID
        """
        if not repo_path.exists():
            raise ValueError(f"Repository path does not exist: {repo_path}")

        if name is None:
            name = repo_path.name

        # Add repository to database
        repo_id = await self.db.add_repository(
            str(repo_path.absolute()),
            name,
            {"indexed_files": 0}
        )

        # Find all source files
        source_files = self._find_source_files(repo_path)

        indexed_count = 0
        for file_path in source_files:
            try:
                await self.index_file(repo_id, file_path)
                indexed_count += 1
                if indexed_count % 10 == 0:
                    print(f"Indexed {indexed_count}/{len(source_files)} files...")
            except Exception as e:
                print(f"Error indexing {file_path}: {e}")
                continue

        print(f"Successfully indexed {indexed_count} files")
        return repo_id

    def _find_source_files(self, repo_path: Path) -> list[Path]:
        """Find all source files in the repository."""
        source_files: list[Path] = []

        # Collect all supported extensions
        extensions: set[str] = set()
        for exts in self.parser.LANGUAGE_EXTENSIONS.values():
            extensions.update(exts)

        # Common directories to skip
        skip_dirs = {
            ".git", "node_modules", "__pycache__", "venv", "env",
            ".venv", "dist", "build", "target", ".next", ".nuxt"
        }

        for path in repo_path.rglob("*"):
            # Skip directories
            if path.is_dir():
                continue

            # Skip files in ignored directories
            if any(skip_dir in path.parts for skip_dir in skip_dirs):
                continue

            # Check if file has supported extension
            if path.suffix.lower() in extensions:
                # Check file size
                if path.stat().st_size <= self.config.max_file_size:
                    source_files.append(path)

        return source_files

    async def index_file(self, repo_id: int, file_path: Path) -> int:
        """
        Index a single file.

        Returns:
            File ID
        """
        # Parse the file
        try:
            ast_root, content, file_hash = self.parser.parse_file(file_path)
            language = self.parser.detect_language(file_path)
        except Exception as e:
            raise ValueError(f"Failed to parse file: {e}") from e

        if not self.db.conn:
            raise RuntimeError("Database not connected")

        # Insert file record
        async with self.db.conn.cursor() as cur:
            await cur.execute(
                """
                INSERT INTO files (repository_id, path, language, content, hash, metadata)
                VALUES (%s, %s, %s, %s, %s, %s)
                ON CONFLICT (repository_id, path) DO UPDATE
                SET content = EXCLUDED.content,
                    hash = EXCLUDED.hash,
                    indexed_at = CURRENT_TIMESTAMP
                RETURNING id
                """,
                (repo_id, str(file_path), language, content, file_hash, json.dumps({})),
            )
            result = await cur.fetchone()
            file_id = result["id"] if result else -1

        # Index AST nodes
        await self._index_ast_nodes(file_id, ast_root)

        # Extract and index symbols
        if language:
            symbols = self.parser.extract_symbols(ast_root, language)
            await self._index_symbols(file_id, symbols)

            # Extract and index references
            references = self.parser.extract_references(ast_root, symbols, language)
            await self._index_references(file_id, symbols, references)

            # Index code chunks
            await self._index_code_chunks(file_id, symbols)

        await self.db.conn.commit()
        return file_id

    async def _index_ast_nodes(self, file_id: int, ast_root: ASTNode) -> None:
        """Index AST nodes recursively."""
        if not self.db.conn:
            return

        node_id_map: dict[ASTNode, int] = {}

        async def insert_node(node: ASTNode, parent_db_id: Optional[int]) -> int:
            async with self.db.conn.cursor() as cur:  # type: ignore
                await cur.execute(
                    """
                    INSERT INTO ast_nodes
                    (file_id, parent_id, node_type, text, start_line, start_column, end_line, end_column)
                    VALUES (%s, %s, %s, %s, %s, %s, %s, %s)
                    RETURNING id
                    """,
                    (
                        file_id,
                        parent_db_id,
                        node.node_type,
                        node.text[:1000],  # Limit text length
                        node.start_line,
                        node.start_column,
                        node.end_line,
                        node.end_column,
                    ),
                )
                result = await cur.fetchone()
                node_db_id = result["id"] if result else -1
                node_id_map[node] = node_db_id

                # Recursively insert children
                for child in node.children:
                    await insert_node(child, node_db_id)

                return node_db_id

        await insert_node(ast_root, None)

    async def _index_symbols(self, file_id: int, symbols: list[Symbol]) -> None:
        """Index symbols with embeddings."""
        if not self.db.conn:
            return

        for symbol in symbols:
            # Generate embedding
            embedding = self.embedder.encode_symbol(
                symbol.name,
                symbol.signature,
                symbol.docstring,
            )

            async with self.db.conn.cursor() as cur:
                await cur.execute(
                    """
                    INSERT INTO symbols
                    (file_id, name, kind, signature, docstring, start_line, end_line, embedding)
                    VALUES (%s, %s, %s, %s, %s, %s, %s, %s)
                    ON CONFLICT DO NOTHING
                    """,
                    (
                        file_id,
                        symbol.name,
                        symbol.kind,
                        symbol.signature,
                        symbol.docstring,
                        symbol.start_line,
                        symbol.end_line,
                        embedding.tolist(),
                    ),
                )

    async def _index_references(
        self,
        file_id: int,
        symbols: list[Symbol],
        references: list[dict[str, Any]]
    ) -> None:
        """Index references between symbols."""
        if not self.db.conn:
            return

        # Build symbol name to ID mapping
        symbol_map: dict[str, int] = {}
        async with self.db.conn.cursor() as cur:
            await cur.execute(
                "SELECT id, name FROM symbols WHERE file_id = %s",
                (file_id,),
            )
            rows = await cur.fetchall()
            for row in rows:
                symbol_map[row["name"]] = row["id"]

        # Insert references
        for ref in references:
            ref_name = ref["name"]
            if ref_name not in symbol_map:
                continue

            # For now, we don't know the "from" symbol, so we'll create self-references
            # In a more sophisticated implementation, we'd track context
            symbol_id = symbol_map[ref_name]

            async with self.db.conn.cursor() as cur:
                await cur.execute(
                    """
                    INSERT INTO references
                    (from_symbol_id, to_symbol_id, reference_type, file_id, line, column)
                    VALUES (%s, %s, %s, %s, %s, %s)
                    ON CONFLICT DO NOTHING
                    """,
                    (
                        symbol_id,
                        symbol_id,
                        ref["type"],
                        file_id,
                        ref["line"],
                        ref["column"],
                    ),
                )

    async def _index_code_chunks(self, file_id: int, symbols: list[Symbol]) -> None:
        """Index code chunks with embeddings."""
        if not self.db.conn:
            return

        # Get symbol IDs
        symbol_map: dict[str, int] = {}
        async with self.db.conn.cursor() as cur:
            await cur.execute(
                "SELECT id, name FROM symbols WHERE file_id = %s",
                (file_id,),
            )
            rows = await cur.fetchall()
            for row in rows:
                symbol_map[row["name"]] = row["id"]

        # Create chunks from symbols
        for symbol in symbols:
            embedding = self.embedder.encode_code_chunk(symbol.text)
            symbol_id = symbol_map.get(symbol.name)

            async with self.db.conn.cursor() as cur:
                await cur.execute(
                    """
                    INSERT INTO code_chunks
                    (file_id, symbol_id, text, start_line, end_line, embedding)
                    VALUES (%s, %s, %s, %s, %s, %s)
                    """,
                    (
                        file_id,
                        symbol_id,
                        symbol.text[:5000],  # Limit chunk size
                        symbol.start_line,
                        symbol.end_line,
                        embedding.tolist(),
                    ),
                )

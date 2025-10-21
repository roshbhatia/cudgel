"""Configuration management for cudgel."""

from pathlib import Path
from typing import Optional

from pydantic import Field
from pydantic_settings import BaseSettings, SettingsConfigDict


class CudgelConfig(BaseSettings):
    """Cudgel configuration settings."""

    model_config = SettingsConfigDict(
        env_file=".env",
        env_prefix="CUDGEL_",
        case_sensitive=False,
    )

    # Database settings
    db_host: str = Field(default="localhost", description="PostgreSQL host")
    db_port: int = Field(default=5432, description="PostgreSQL port")
    db_name: str = Field(default="cudgel", description="Database name")
    db_user: str = Field(default="cudgel", description="Database user")
    db_password: str = Field(default="cudgel", description="Database password")

    # Temporal settings
    temporal_host: str = Field(default="localhost:7233", description="Temporal server address")
    temporal_namespace: str = Field(default="default", description="Temporal namespace")
    temporal_task_queue: str = Field(default="cudgel-indexing", description="Temporal task queue")

    # Embedding settings
    embedding_model: str = Field(
        default="sentence-transformers/all-MiniLM-L6-v2",
        description="Sentence transformer model for embeddings"
    )
    embedding_dimension: int = Field(default=384, description="Embedding vector dimension")

    # LSP settings
    lsp_port: int = Field(default=6010, description="LSP server port")

    # Indexing settings
    index_batch_size: int = Field(default=100, description="Batch size for indexing")
    max_file_size: int = Field(default=1024 * 1024, description="Maximum file size to index (bytes)")

    @property
    def database_url(self) -> str:
        """Get PostgreSQL connection URL."""
        return f"postgresql://{self.db_user}:{self.db_password}@{self.db_host}:{self.db_port}/{self.db_name}"


def get_config() -> CudgelConfig:
    """Get the global configuration instance."""
    return CudgelConfig()

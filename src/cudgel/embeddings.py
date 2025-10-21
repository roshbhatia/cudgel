"""Embedding generation for code and text."""

from typing import Optional

import numpy as np
from sentence_transformers import SentenceTransformer

from cudgel.config import CudgelConfig


class EmbeddingGenerator:
    """Generate embeddings for code and text using sentence transformers."""

    def __init__(self, config: CudgelConfig):
        self.config = config
        self.model: Optional[SentenceTransformer] = None

    def load_model(self) -> None:
        """Load the sentence transformer model."""
        if self.model is None:
            self.model = SentenceTransformer(self.config.embedding_model)

    def encode(self, text: str | list[str]) -> np.ndarray:
        """
        Generate embeddings for text.

        Args:
            text: Single text string or list of texts

        Returns:
            Numpy array of embeddings
        """
        if self.model is None:
            self.load_model()

        assert self.model is not None
        embeddings = self.model.encode(text, convert_to_numpy=True)
        return embeddings

    def encode_symbol(self, symbol_name: str, signature: Optional[str], docstring: Optional[str]) -> np.ndarray:
        """
        Generate embedding for a code symbol.

        Combines name, signature, and docstring for better semantic representation.
        """
        parts = [symbol_name]
        if signature:
            parts.append(signature)
        if docstring:
            parts.append(docstring)

        text = " ".join(parts)
        return self.encode(text)

    def encode_code_chunk(self, code: str) -> np.ndarray:
        """Generate embedding for a code chunk."""
        return self.encode(code)

    def encode_query(self, query: str) -> np.ndarray:
        """Generate embedding for a natural language query."""
        return self.encode(query)

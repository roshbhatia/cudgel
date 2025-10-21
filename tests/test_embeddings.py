"""Tests for embeddings."""

import pytest
import numpy as np
from cudgel.config import CudgelConfig
from cudgel.embeddings import EmbeddingGenerator


@pytest.fixture
def embedder():
    """Create an embedding generator."""
    config = CudgelConfig()
    return EmbeddingGenerator(config)


def test_encode_text(embedder):
    """Test encoding text."""
    embedder.load_model()

    text = "This is a test function"
    embedding = embedder.encode(text)

    assert isinstance(embedding, np.ndarray)
    assert len(embedding.shape) == 1  # 1D array
    assert embedding.shape[0] == 384  # Default dimension


def test_encode_multiple_texts(embedder):
    """Test encoding multiple texts."""
    embedder.load_model()

    texts = ["function one", "function two", "function three"]
    embeddings = embedder.encode(texts)

    assert isinstance(embeddings, np.ndarray)
    assert embeddings.shape == (3, 384)


def test_encode_symbol(embedder):
    """Test encoding a symbol."""
    embedder.load_model()

    embedding = embedder.encode_symbol(
        "calculate_total",
        "def calculate_total(items):",
        "Calculate the total price of items"
    )

    assert isinstance(embedding, np.ndarray)
    assert embedding.shape[0] == 384


def test_similar_embeddings(embedder):
    """Test that similar texts have similar embeddings."""
    embedder.load_model()

    # Similar texts
    text1 = "parse configuration file"
    text2 = "read config file"

    # Different text
    text3 = "calculate mathematical formula"

    emb1 = embedder.encode(text1)
    emb2 = embedder.encode(text2)
    emb3 = embedder.encode(text3)

    # Cosine similarity
    def cosine_similarity(a, b):
        return np.dot(a, b) / (np.linalg.norm(a) * np.linalg.norm(b))

    sim_12 = cosine_similarity(emb1, emb2)
    sim_13 = cosine_similarity(emb1, emb3)

    # Similar texts should have higher similarity
    assert sim_12 > sim_13

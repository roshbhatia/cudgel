"""Pytest configuration and fixtures."""

import pytest
from cudgel.config import CudgelConfig


@pytest.fixture
def test_config():
    """Provide a test configuration."""
    return CudgelConfig(
        db_host="localhost",
        db_port=5432,
        db_name="cudgel_test",
        db_user="cudgel",
        db_password="cudgel",
        embedding_model="sentence-transformers/all-MiniLM-L6-v2",
        embedding_dimension=384,
    )

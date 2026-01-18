"""Tests for the DimensionRegistry class."""

from inchworm.dimensions import DimensionRegistry

def test_dimension_registry_initialization():
    """
    WHEN a DimensionRegistry is initialized,
    THEN it should create an instance without errors.
    """
    _ = DimensionRegistry()

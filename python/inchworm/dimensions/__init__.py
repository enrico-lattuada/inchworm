"""Dimensions submodule for inchworm."""

from ..inchworm import dimensions as _dimensions

DimensionRegistry = _dimensions.DimensionRegistry
BaseDimensionDef = _dimensions.BaseDimensionDef

__all__ = [
    "BaseDimensionDef",
    "DimensionRegistry",
]

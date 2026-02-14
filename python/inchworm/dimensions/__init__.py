"""Dimensions submodule for inchworm."""

from ..inchworm import dimensions as _dimensions

DimensionRegistry = _dimensions.DimensionRegistry
BaseDimensionDef = _dimensions.BaseDimensionDef
DerivedDimensionDef = _dimensions.DerivedDimensionDef

__all__ = [
    "BaseDimensionDef",
    "DerivedDimensionDef",
    "DimensionRegistry",
]

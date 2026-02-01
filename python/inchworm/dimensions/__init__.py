"""Dimensions submodule for inchworm."""

from collections.abc import Mapping
from ..inchworm import dimensions as _dimensions

DimensionRegistry = _dimensions.DimensionRegistry
BaseDimensionDef = _dimensions.BaseDimensionDef
BaseDimensionsView = _dimensions.BaseDimensionsView

Mapping.register(BaseDimensionsView)

__all__ = [
    "BaseDimensionDef",
    "DimensionRegistry",
    "BaseDimensionsView",
]

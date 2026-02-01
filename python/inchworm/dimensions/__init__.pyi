"""inchworm.dimensions module."""

from typing import Iterator, Mapping

class DimensionRegistry:
    """A registry for managing dimensions.

    `DimensionRegistry` provides a central location to define and manage
    physical dimensions (e.g., length, mass, time) that form the foundation
    of the units system.
    """

    def __init__(self) -> None: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
    def base_dimensions(self) -> BaseDimensionsView:
        """Get the base dimensions in the registry.

        Returns
        -------
        base_dimensions : BaseDimensionsView
            A mapping of base dimension names to their definitions.
        """
        ...

    def try_insert_new_base_dimension(
        self, dimension: str, definition: BaseDimensionDef
    ) -> None:
        """Try to insert a new base dimension.

        This method attempts to insert a new base dimension into the registry.
        Raises an error if the dimension already exists.

        Parameters
        ----------
        dimension : str
            The name of the base dimension to insert.
        definition : BaseDimensionDef
            The definition of the base dimension.

        Raises
        ------
        ValueError
            If the base dimension already exists in the registry.
        """
        ...

    def replace_base_dimension(
        self, dimension: str, definition: BaseDimensionDef
    ) -> BaseDimensionDef | None:
        """Replace an existing base dimension.

        This method replaces the definition of an existing base dimension in
        the registry. If the dimension does not exist, it is added.

        Parameters
        ----------
        dimension : str
            The name of the base dimension to replace.
        definition : BaseDimensionDef
            The new definition of the base dimension.

        Returns
        -------
        previous_definition : BaseDimensionDef | None
            The previous definition of the base dimension if it existed,
            otherwise `None`.
        """
        ...

class BaseDimensionDef:
    """Definition of a base physical dimension.

    `BaseDimensionDef` represents a fundamental physical dimension, such as
    length, mass, or time. These base dimensions serve as the building blocks
    for defining derived dimensions and units.
    """

    def __init__(self, name: str, symbol: str) -> None: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
    @property
    def name(self) -> str:
        """The name of the base dimension.

        Returns
        -------
        name : str
            The name of the base dimension.
        """
        ...

    @property
    def symbol(self) -> str:
        """The symbol of the base dimension.

        Returns
        -------
        symbol : str
            The symbol of the base dimension.
        """
        ...

class BaseDimensionsView(Mapping[str, BaseDimensionDef]):
    """A view of the base dimensions in a DimensionRegistry.

    `BaseDimensionsView` provides a read-only mapping interface to access
    the base dimensions defined in a `DimensionRegistry`.
    """

    def __getitem__(self, key: str) -> BaseDimensionDef: ...
    def __iter__(self) -> Iterator[str]: ...
    def __len__(self) -> int: ...
    def keys(self) -> Iterator[str]: ...
    def values(self) -> Iterator[BaseDimensionDef]: ...
    def items(self) -> Iterator[tuple[str, BaseDimensionDef]]: ...
    def get(
        self, key: str, default: BaseDimensionDef | None = None
    ) -> BaseDimensionDef | None: ...

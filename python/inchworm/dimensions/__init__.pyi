"""inchworm.dimensions module."""

class DimensionRegistry:
    """A registry for managing dimensions.

    `DimensionRegistry` provides a central location to define and manage
    physical dimensions (e.g., length, mass, time) that form the foundation
    of the units system.

    Examples
    --------
    >>> from inchworm.dimensions import DimensionRegistry
    >>> registry = DimensionRegistry()
    """

    def __init__(self) -> None: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class BaseDimensionDef:
    """A definition of a base physical dimension.

    `BaseDimensionDef` represents a fundamental physical dimension, such as
    length, mass, or time, that form the basis for derived dimensions in a
    units system.

    Attributes
    ----------
    name : str
        The name of the base dimension.
    symbol : str
        The symbol of the base dimension.

    Examples
    --------
    >>> from inchworm.dimensions import BaseDimensionDef
    >>> length_def = BaseDimensionDef(name="length", symbol="L")
    """

    def __init__(self, name: str, symbol: str) -> None:
        """Initialize a BaseDimensionDef.

        Parameters
        ----------
        name : str
            The name of the base dimension.
        symbol : str
            The symbol of the base dimension.
        """
        ...

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

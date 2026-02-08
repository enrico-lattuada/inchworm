"""Tests for the BaseDimensionDef class."""

from inchworm.dimensions import BaseDimensionDef
from pytest_cases import parametrize_with_cases


class BaseDimensionDefInitCases:
    """Test cases for BaseDimensionDef initialization."""

    def case_valid_parameters(self) -> tuple[str, str]:
        """A valid name and symbol."""
        return "Length", "L"


@parametrize_with_cases(
    "name, symbol",
    cases=BaseDimensionDefInitCases,
)
def test_initialization(name, symbol):
    """
    GIVEN valid parameters for a BaseDimensionDef,
    WHEN a BaseDimensionDef is initialized,
    THEN it should create an instance without errors.
    """
    _ = BaseDimensionDef(name=name, symbol=symbol)


class BaseDimensionDefGetCases:
    """Test cases for BaseDimensionDef .name and .symbol."""

    def case_valid_parameters(self) -> tuple[BaseDimensionDef, str, str]:
        """A valid name and symbol."""
        name = "Length"
        symbol = "L"
        dimension_def = BaseDimensionDef(name=name, symbol=symbol)
        return dimension_def, name, symbol


@parametrize_with_cases(
    "dimension_def, name, symbol",
    cases=BaseDimensionDefGetCases,
)
def test_get_name(dimension_def, name, symbol):
    """
    GIVEN a BaseDimensionDef instance,
    WHEN accessing the .name attribute,
    THEN it should return the correct name.
    """
    del symbol  # Unused in this test
    assert dimension_def.name == name, (
        f"Expected name '{name}', got '{dimension_def.name}'"
    )


@parametrize_with_cases(
    "dimension_def, name, symbol",
    cases=BaseDimensionDefGetCases,
)
def test_get_symbol(dimension_def, name, symbol):
    """
    GIVEN a BaseDimensionDef instance,
    WHEN accessing the .symbol attribute,
    THEN it should return the correct symbol.
    """
    del name  # Unused in this test
    assert dimension_def.symbol == symbol, (
        f"Expected symbol '{symbol}', got '{dimension_def.symbol}'"
    )

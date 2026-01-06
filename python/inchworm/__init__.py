from .inchworm import add
from importlib.metadata import version, metadata, PackageNotFoundError

try:
    __version__ = version("inchworm")
    _metadata = metadata("inchworm")
    __doc__ = _metadata.get("Summary", "")
    __license__ = _metadata.get("License-Expression", "")
    _author_email = _metadata.get("Author-email")
    if _author_email is not None:
        # Parse "Name <email>, Name2 <email2>" format
        __author__ = tuple(
            author.split(" <")[0].strip() for author in _author_email.split(",")
        )
    else:
        __author__ = ()
except PackageNotFoundError:
    __version__ = "0+unknown"
    __doc__ = ""
    __license__ = ""
    __author__ = ()

__all__ = ["add"]

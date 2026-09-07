"""openparkan -- tools for reading the data files of Parkan: Iron Strategy (1998).

This package is a clean-room reader built by observing the shipped data files.
It contains no game assets; point it at your own installation.
"""

__version__ = "0.1.0"

from . import landmesh, nres, png, texm  # noqa: F401

"""Which textures the engine uploads with an alpha channel."""

from __future__ import annotations

from openparkan import texm


def test_only_the_alpha_formats_and_two_header_bits_get_alpha():
    assert texm.uploads_with_alpha(texm.FMT_ARGB4444)
    assert texm.uploads_with_alpha(texm.FMT_ARGB8888)
    assert not texm.uploads_with_alpha(texm.FMT_PALETTE8)
    assert not texm.uploads_with_alpha(texm.FMT_RGB565, 0x4000000)
    assert texm.uploads_with_alpha(texm.FMT_PALETTE8, texm.ALPHA_SURFACE)
    assert texm.uploads_with_alpha(texm.FMT_XRGB8888, texm.FADE_PALETTE)

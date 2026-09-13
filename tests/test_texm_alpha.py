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


def test_an_opaque_load_drops_the_alpha_surface():
    assert not texm.uploads_with_alpha(texm.FMT_ARGB8888, load_flags=texm.LOAD_OPAQUE)
    assert not texm.uploads_with_alpha(texm.FMT_PALETTE8, texm.ALPHA_SURFACE, texm.LOAD_OPAQUE)
    assert texm.uploads_with_alpha(texm.FMT_ARGB8888, load_flags=texm.LOAD_BIT0_MATERIAL)


def test_a_lit_skin_loads_opaque_unless_emboss_bump_is_on():
    assert texm.material_load_flags(2) == texm.LOAD_OPAQUE
    assert texm.material_load_flags(2, emboss_bump=True) == 0
    assert texm.material_load_flags(4) == 0
    assert texm.material_load_flags(8) == 0
    assert texm.material_load_flags(5) == texm.LOAD_BIT0_MATERIAL

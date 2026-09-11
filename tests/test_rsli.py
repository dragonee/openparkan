"""The two algorithms in the RsLi reader, where a slip would be silent."""

from __future__ import annotations

import pytest

from openparkan import rsli


@pytest.mark.parametrize("seed", [0, 1, 0x1234, 0xDEADBEEF, 0xFFFFFFFF])
def test_the_table_cipher_is_its_own_inverse(seed):
    plain = bytes(range(256)) * 3
    once = rsli.decrypt_table(plain, seed)
    assert once != plain or seed == 0
    assert rsli.decrypt_table(once, seed) == plain


def test_the_keystream_depends_on_the_seed():
    plain = b"\x00" * 64
    assert rsli.decrypt_table(plain, 1) != rsli.decrypt_table(plain, 2)


def test_an_empty_table_decrypts_to_nothing():
    assert rsli.decrypt_table(b"", 0x1234) == b""


def test_lzss_returns_what_it_is_asked_for():
    # one literal run: a control byte with every bit set, then eight bytes
    data = bytes([0xFF]) + b"openpark"
    assert rsli.unpack_lzss(data, 8) == b"openpark"


def test_lzss_repeats_from_its_window():
    # eight literals, then a control byte whose low bit clears for a back
    # reference to what was just written
    literals = bytes([0xFF]) + b"abcdefgh"
    start = rsli.LZSS_START
    ref = ((start) & 0xFF, ((start >> 4) & 0xF0) | 1)   # offset, length 3+1
    out = rsli.unpack_lzss(literals + bytes([0xFE]) + bytes(ref) + b"z", 13)
    assert out.startswith(b"abcdefgh")
    assert len(out) == 13


def test_is_rsli_says_no_to_a_plain_file(tmp_path):
    path = tmp_path / "not.lib"
    path.write_bytes(b"NRes" + bytes(60))
    assert rsli.is_rsli(path) is False

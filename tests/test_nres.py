"""The container every other reader sits on."""

from __future__ import annotations

import struct

import pytest

from openparkan import nres


def test_reads_back_what_was_written(nres_archive):
    data = nres_archive([
        ("MESH", "first.msh", b"\x01\x02\x03\x04"),
        ("WEA ", "second.wea", b"hello world"),
    ])
    archive = nres.NResArchive(data)
    assert len(archive) == 2
    assert [e.name for e in archive] == ["first.msh", "second.wea"]
    assert [e.tag for e in archive] == ["MESH", "WEA"]
    assert archive.read_name("first.msh") == b"\x01\x02\x03\x04"
    assert archive.read_name("second.wea") == b"hello world"


def test_an_empty_archive_is_still_an_archive(nres_archive):
    archive = nres.NResArchive(nres_archive([]))
    assert len(archive) == 0
    assert list(archive) == []


def test_rejects_a_file_without_the_magic():
    with pytest.raises(nres.NotAnNResArchive):
        nres.NResArchive(b"NOPE" + bytes(60))


def test_rejects_a_declared_size_that_is_a_lie(nres_archive):
    data = bytearray(nres_archive([("MESH", "a.msh", b"xyz")]))
    struct.pack_into("<I", data, 12, len(data) + 1)
    with pytest.raises(ValueError, match="header declares"):
        nres.NResArchive(bytes(data))


def test_rejects_a_directory_that_cannot_fit(nres_archive):
    data = bytearray(nres_archive([]))
    struct.pack_into("<I", data, 8, 1000)
    with pytest.raises(ValueError, match="does not fit"):
        nres.NResArchive(bytes(data))


def test_a_numeric_tag_reads_as_a_number(nres_archive):
    data = nres_archive([("\x07\x00\x00\x00", "stream", b"")])
    entry = next(iter(nres.NResArchive(data)))
    assert entry.tag == "#7"
    assert entry.type_id == 7


def test_missing_member_raises(nres_archive):
    archive = nres.NResArchive(nres_archive([("MESH", "a.msh", b"x")]))
    with pytest.raises(KeyError):
        archive.find("nope.msh")

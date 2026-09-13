"""A mission clan's trailing word is how many bots it may field."""

from __future__ import annotations

from openparkan import mission


def test_the_word_after_the_tree_is_the_mind_count():
    clan = mission.Clan("Player", 1, (0.0, 0.0), "", "", unknown=(-1, 5))
    assert clan.minds == 5

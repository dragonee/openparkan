"""A mission clan's trailing word is how many bots it may field."""

from __future__ import annotations

from openparkan import mission


def test_the_word_after_the_tree_is_the_mind_count():
    clan = mission.Clan("Player", 1, (0.0, 0.0), "", "", unknown=(-1, 5))
    assert clan.minds == 5


def test_the_word_after_the_base_is_the_clan_type():
    animals = mission.Clan("Anml", 0, (0.0, 0.0), "", "")
    neutral = mission.Clan("Ntrl", 3, (0.0, 0.0), "", "")
    assert animals.type == mission.CLAN_NATURE
    assert neutral.type == mission.CLAN_NEUTRAL

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


def _mission(clans):
    return mission.Mission(source=None, version=1, routes=[], clans=clans, objects=[],
                           map_path="", description="", viewpoints=[],
                           unknown_pre_objects=0)


def test_relation_words_pass_through_and_a_clan_is_its_own_ally():
    player = mission.Clan("Plr", mission.CLAN_PLAYER, (0.0, 0.0), "", "",
                          relations={"Plr": 1, "enm": 0, "Ally": 2})
    enemy = mission.Clan("Enm", mission.CLAN_ENEMY, (0.0, 0.0), "", "",
                         relations={"Plr": 0, "Enm": 1, "Ally": 0})
    ally = mission.Clan("Ally", mission.CLAN_ENEMY, (0.0, 0.0), "", "",
                        relations={"Plr": 2, "Enm": 0, "Ally": 1})
    matrix = _mission([player, enemy, ally]).relations()
    assert matrix[0] == [mission.RELATION_ALLIED, mission.RELATION_HOSTILE,
                         mission.RELATION_ALLIED]
    assert matrix[1][1] == mission.RELATION_ALLIED      # the file's 1 towards itself
    assert matrix[2][0] == mission.RELATION_ALLIED


def test_a_neutral_clan_is_neutral_both_ways_and_an_unnamed_clan_is_hostile():
    player = mission.Clan("Plr", mission.CLAN_PLAYER, (0.0, 0.0), "", "",
                          relations={"Plr": 1, "Ntrl": 0})
    neutral = mission.Clan("Ntrl", mission.CLAN_NEUTRAL, (0.0, 0.0), "", "",
                           relations={"Plr": 0})
    animals = mission.Clan("Anml", mission.CLAN_NATURE, (0.0, 0.0), "", "")
    matrix = _mission([player, neutral, animals]).relations()
    assert matrix[0][1] == matrix[1][0] == mission.RELATION_NEUTRAL
    assert matrix[1][1] == mission.RELATION_ALLIED
    assert matrix[0][2] == mission.RELATION_HOSTILE


def test_a_marker_is_light_blue_for_its_own_clan_grey_neutral_and_by_relation_otherwise():
    player = mission.Clan("Plr", mission.CLAN_PLAYER, (0.0, 0.0), "", "",
                          relations={"Plr": 1, "Trgt": 1, "Enm": 0, "Ally": 2})
    dummies = mission.Clan("Trgt", mission.CLAN_ENEMY, (0.0, 0.0), "", "",
                           relations={"Plr": 1})
    enemy = mission.Clan("Enm", mission.CLAN_ENEMY, (0.0, 0.0), "", "",
                         relations={"Plr": 0})
    neutral = mission.Clan("Ntrl", mission.CLAN_NEUTRAL, (0.0, 0.0), "", "")
    animals = mission.Clan("Anml", mission.CLAN_NATURE, (0.0, 0.0), "", "",
                           relations={"Plr": 0})
    ally = mission.Clan("Ally", mission.CLAN_ENEMY, (0.0, 0.0), "", "",
                        relations={"Plr": 2})
    m = _mission([player, dummies, enemy, neutral, animals, ally])
    assert [m.marker_colour(0, c) for c in range(6)] == [
        (128, 128, 255), (255, 0, 255), (255, 0, 0), (160, 160, 160), (255, 255, 0),
        (0, 255, 255)]

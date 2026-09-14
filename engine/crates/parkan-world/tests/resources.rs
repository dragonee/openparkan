//! A mission's words and voices against the install: `cargo test -- --ignored`.

use parkan_formats::gamedir;
use parkan_world::resources::{self, Messages, Sounds};

/// A RIFF WAVE file whose declared size is the bytes it holds.
fn is_wave(bytes: &[u8]) -> bool {
    bytes.len() > 44
        && &bytes[..4] == b"RIFF"
        && &bytes[8..12] == b"WAVE"
        && u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize + 8 == bytes.len()
}

#[test]
#[ignore = "needs the game install"]
fn mission_01s_objectives_messages_and_sounds_resolve() {
    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let dir = gamedir::resolve(&game, gamedir::MISSION_01).unwrap();

    // Three primary objectives and no bonus (docs/34, "Objectives and the end of a mission").
    let objectives = resources::objectives(&dir).unwrap();
    let texts: Vec<(&str, bool)> = objectives.iter().map(|o| (o.text.as_str(), o.exempt)).collect();
    assert_eq!(
        texts,
        [
            ("1. Destroy all the targets on the island", false),
            ("2. Capture the neutral warbots", false),
            ("3. Destroy the enemy warbot", false)
        ]
    );

    // Ids 0 to 21, skipping 6 (docs/21): the briefing's lines, then the script's.
    let messages = Messages::load(&game, &dir).unwrap();
    let ids: Vec<i64> = messages.0.iter().map(|m| m.index).collect();
    assert_eq!(ids, (0..=21).filter(|&i| i != 6).collect::<Vec<i64>>());
    let text = resources::text_resources(&game).unwrap();
    for m in &messages.0 {
        assert!(m.text.is_some(), "message {} has no text", m.index);
        let voice = m.voice.as_ref().unwrap_or_else(|| panic!("message {} has no voice", m.index));
        assert!(voice.library.ends_with("voices.lib"), "message {} speaks from {:?}", m.index, voice.library);
    }
    let welcome = messages.get(11).unwrap();
    assert_eq!((welcome.text_id.as_str(), welcome.text.as_deref()), ("T01_I01", text.get("T01_I01")));
    let voice = welcome.voice.as_ref().unwrap();
    assert!(voice.member.eq_ignore_ascii_case("T01_I01.wav"));
    assert!(is_wave(&voice.read().unwrap()), "T01_I01.wav is a whole WAVE file");

    // The game's own voices and words for a completion (docs/34).
    let sounds = Sounds::open(&game, &dir).unwrap();
    let done = sounds.get("VOICE_OBJ_COMPLETE").unwrap();
    assert!(done.member.eq_ignore_ascii_case("vc_obj_cpl.wav") && is_wave(&done.read().unwrap()));
    assert!(sounds.get("VOICE_MISSION_COMPLETE").unwrap().member.eq_ignore_ascii_case("vc_mis_cpl.wav"));
    let strings = resources::game_strings(&game).unwrap();
    assert_eq!(strings.get(&5040).map(String::as_str), Some("Objective is completed"));
    assert_eq!(strings.get(&6170).map(String::as_str), Some("Recieved message is already in history"));

    // The theme loops from sounds.lib; the variations are the day's birds and the night's frogs.
    let ambient = resources::ambient(&game, &dir).unwrap();
    let theme = ambient.theme.unwrap();
    assert!(theme.library.ends_with("sounds.lib") && theme.member.eq_ignore_ascii_case("atm_c1_lp.wav"));
    assert!(is_wave(&theme.read().unwrap()));
    assert_eq!((ambient.default.len(), ambient.day.len(), ambient.night.len()), (0, 5, 2));
    assert!(ambient.day.iter().chain(&ambient.night).all(|s| s.exists()));
}

//! Against the game's own files. They need an install, so they are ignored by
//! default: `cargo test -- --ignored`, with `PARKAN_DIR` set if the install is
//! not beside this repository.

use parkan_formats::{gamedir, mission, nres};

fn game() -> std::path::PathBuf {
    gamedir::find(None).expect("a Parkan install: set PARKAN_DIR")
}

#[test]
#[ignore = "needs the game install"]
fn mission_01_places_the_hero_at_its_start() {
    let dir = gamedir::resolve(&game(), gamedir::MISSION_01).expect("Mission 01");
    let data = std::fs::read(dir.join("data.tma")).unwrap();
    let m = mission::parse(&data, "Mission.01").unwrap();
    assert_eq!((m.objects.len(), m.clans.len()), (33, 4));
    assert_eq!(m.map_name(), "Tut_1");
    let hero =
        m.objects.iter().find(|o| o.path.to_ascii_uppercase().ends_with("\\HERO\\TUT1_P.DAT")).unwrap();
    let [x, y, z] = hero.position;
    assert!((x - 433.0).abs() < 0.1 && (y - 477.1).abs() < 0.1 && (z - 14.1).abs() < 0.1);
    assert!((hero.rotation + 1.639).abs() < 1e-3);
}

#[test]
#[ignore = "needs the game install"]
fn every_archive_in_the_install_opens() {
    let mut opened = 0;
    let mut stack = vec![game()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(dir).unwrap().filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if std::fs::read(&path).is_ok_and(|d| d.starts_with(nres::MAGIC)) {
                nres::Archive::open(&path).unwrap_or_else(|e| panic!("{e}"));
                opened += 1;
            }
        }
    }
    assert_eq!(opened, 120);
}

/// Every `Tfnt` the install ships: `gamefont.rlb`'s and `sys.lib`'s `ARIALTEX.TFT` and
/// the nine of `ui/font.lib`. See `docs/12-rsli.md`, "The header's four words are the
/// font's metrics".
#[test]
#[ignore = "needs the game install"]
fn all_eleven_fonts_step_one_past_the_advance_and_draw_one_texel_to_the_pixel() {
    use parkan_formats::{font, rsli, texm};

    let mut fonts: Vec<(String, font::Font, texm::Texture)> = Vec::new();
    let archive = rsli::Archive::open(&gamedir::resolve(&game(), "gamefont.rlb").unwrap()).unwrap();
    let tft = font::parse_font(&archive.read_name("ARIALTEX.TFT").unwrap(), "ARIALTEX.TFT").unwrap();
    let pal = font::parse_palette(&archive.read_name("PAL.PAL").unwrap(), "PAL.PAL").unwrap();
    let atlas = tft.decode_atlas(&pal, "ARIALTEX.TFT").unwrap();
    fonts.push(("gamefont.rlb::ARIALTEX.TFT".into(), tft, atlas));
    for lib in ["sys.lib", "ui/font.lib"] {
        let a = nres::Archive::open(&gamedir::resolve(&game(), lib).unwrap()).unwrap();
        for e in a.entries.iter().filter(|e| e.name.to_ascii_uppercase().ends_with(".TFT")) {
            let tft = font::parse_font(a.read(e).unwrap(), &e.name).unwrap();
            let atlas = texm::decode(&tft.atlas, &e.name, None).unwrap();
            fonts.push((format!("{lib}::{}", e.name), tft, atlas));
        }
    }
    assert_eq!(fonts.len(), 11);

    for (name, tft, atlas) in &fonts {
        // The header's spacing is 1 everywhere, and its v span is the drawn cell.
        assert_eq!(tft.spacing, 1, "{name}");
        let cell = (tft.height + 1) as f32;
        assert!((tft.v_span * atlas.height as f32 - cell).abs() < 1e-3, "{name}");
        // Every drawn glyph spans exactly advance + 1 texels, so the cell tiles the line.
        let drawn: Vec<&font::Glyph> = tft.glyphs.iter().filter(|g| g.drawn()).collect();
        assert!(drawn.len() >= 123, "{name}: {} drawn", drawn.len());
        for g in &drawn {
            let span = (g.u1 - g.u0) * atlas.width as f32;
            assert!((span - (g.advance + 1) as f32).abs() < 1e-2, "{name}: {span} for {}", g.advance);
        }
        // The rows are packed at the cell height: no gap, no overlap.
        let mut rows: Vec<f32> = drawn.iter().map(|g| g.v0 * atlas.height as f32).collect();
        rows.sort_by(f32::total_cmp);
        rows.dedup_by(|a, b| (*a - *b).abs() < 1e-3);
        let pitch = rows.windows(2).map(|w| w[1] - w[0]).fold(f32::MAX, f32::min);
        assert!((pitch - cell).abs() < 1e-2, "{name}: rows {pitch} apart for a cell of {cell}");
        // The atlas is keyed on palette index 0, which its alpha-surface bit clears.
        assert_eq!(atlas.flags14 & texm::ALPHA_SURFACE, texm::ALPHA_SURFACE, "{name}");
        assert!(atlas.levels[0].as_chunks::<4>().0.iter().any(|p| p[3] == 0), "{name}: something is keyed");
        assert!(
            atlas.levels[0].as_chunks::<4>().0.iter().any(|p| p[3] == 255),
            "{name}: and something is not"
        );
    }

    // The two ARIALTEX.TFT are laid out in CP866 and the nine ui fonts in Windows-1251.
    for (name, tft, _) in &fonts {
        let hi: Vec<usize> = (0x80..0x100).filter(|&i| tft.glyphs[i].drawn()).collect();
        if name.contains("ARIALTEX") {
            assert!(hi.iter().all(|&i| (0x80..0xB0).contains(&i) || (0xE0..0xF2).contains(&i)), "{name}");
            assert!(hi.contains(&0x80), "{name}: CP866 puts А at 0x80");
        } else {
            assert_eq!(hi.iter().filter(|&&i| i >= 0xC0).count(), 64, "{name}: Windows-1251's А-Я а-я");
            assert!(!hi.contains(&0x81), "{name}: and nothing of CP866's Cyrillic run");
        }
    }
}

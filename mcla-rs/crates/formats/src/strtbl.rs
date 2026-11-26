//! The game's localised string table (`ui/0xa4c1e8fa`, little-endian):
//! `u32 language count, u32 offset per language, u32 version, u32 key count, keys
//! {u32 length, bytes, NUL}`; every language block is `u32 count` followed by
//! `{u32 joaat(key), u16 flags, u32 font name length, font name, u32 UTF-16 length
//! (with terminator), UTF-16LE text, f32 scale x, f32 scale y, u16}` entries.
//! Language 0 is English.

use crate::hash::joaat;
use crate::{bail, Result};
use hashbrown::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::RwLock;

/// The game's language for this run (0 English .. 4 Italian), set once at start from settings.toml `game.language`
/// (the launcher's choice). Every table read after that reads its block (`StringTable::parse`).
static LANGUAGE: AtomicUsize = AtomicUsize::new(0);

/// Set the run's language (the game's number, as the original's console query gives it).
pub fn set_language(lang: usize) {
    LANGUAGE.store(lang, Ordering::Relaxed);
}

/// The run's language (0 English, 1 Spanish, 2 French, 3 German, 4 Italian).
pub fn language() -> usize {
    LANGUAGE.load(Ordering::Relaxed)
}

/// Brazilian Portuguese is on no disc: the run reads the English block (language 0) and this layer replaces (or adds)
/// the entries it has a translation for (`pt-br.tsv`: `key<TAB>text`, a backslash and an n is a line break).
static PORTUGUESE: AtomicBool = AtomicBool::new(false);
static OVERLAY: RwLock<Option<HashMap<u32, String>>> = RwLock::new(None);
/// Cutscene subtitle lines in Portuguese (`pt-br-subtitles.tsv`: `SCENE<TAB>start time<TAB>text`), keyed
/// `SCENE|start time` exactly as the disc's subtitle table writes the start time.
static SUB_OVERLAY: RwLock<Option<HashMap<String, String>>> = RwLock::new(None);

/// The run is in Brazilian Portuguese (`game.language = "portuguese"`).
pub fn portuguese() -> bool {
    PORTUGUESE.load(Ordering::Relaxed)
}

pub fn set_portuguese(on: bool) {
    PORTUGUESE.store(on, Ordering::Relaxed);
}

fn unescape(s: &str) -> String {
    s.replace("\\n", "\n")
}

/// Lay a pt-BR table over the English block of every string table read after this call. `tsv` lines are
/// `key<TAB>text` (a key is hashed like the game does; lines starting with `#` and blank lines are skipped).
/// Returns how many entries were read.
pub fn set_overlay(tsv: &str) -> usize {
    let mut m: HashMap<u32, String> = HashMap::new();
    for line in tsv.trim_start_matches('\u{feff}').lines() {
        let line = line.trim_end_matches('\r');
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once('\t') else { continue };
        if v.is_empty() {
            continue;
        }
        m.insert(joaat(k.trim()), unescape(v));
    }
    let n = m.len();
    if let Ok(mut g) = OVERLAY.write() {
        *g = Some(m);
    }
    n
}

/// Load the pt-BR subtitle lines (see [`SUB_OVERLAY`]); returns how many were read.
pub fn set_subtitle_overlay(tsv: &str) -> usize {
    let mut m: HashMap<String, String> = HashMap::new();
    for line in tsv.trim_start_matches('\u{feff}').lines() {
        let line = line.trim_end_matches('\r');
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let mut it = line.splitn(3, '\t');
        let (Some(scene), Some(start), Some(text)) = (it.next(), it.next(), it.next()) else { continue };
        if text.is_empty() {
            continue;
        }
        m.insert(format!("{}|{}", scene.trim().to_ascii_uppercase(), start.trim()), unescape(text));
    }
    let n = m.len();
    if let Ok(mut g) = SUB_OVERLAY.write() {
        *g = Some(m);
    }
    n
}

/// The pt-BR text of a subtitle line (scene folder name, the line's start time as the table writes it), if there is one.
pub fn subtitle_override(scene: &str, start: &str) -> Option<String> {
    let g = SUB_OVERLAY.read().ok()?;
    g.as_ref()?.get(&format!("{}|{}", scene.to_ascii_uppercase(), start)).cloned()
}

#[derive(Default, Clone)]
pub struct StringTable {
    pub keys: Vec<String>,
    pub langs: Vec<HashMap<u32, String>>,
    /// the language block `get` reads (0 English, 1 Spanish, 2 French, 3 German, 4 Italian)
    pub lang: usize,
}

impl StringTable {
    pub fn parse(d: &[u8]) -> Result<StringTable> {
        let u32at = |o: usize| -> Result<usize> {
            if o + 4 > d.len() {
                bail!("string table truncated");
            }
            Ok(u32::from_le_bytes(d[o..o + 4].try_into().unwrap()) as usize)
        };
        let n = u32at(0)?;
        if n == 0 || n > 64 {
            bail!("not a string table");
        }
        let mut offs = Vec::new();
        for i in 0..n {
            offs.push(u32at(4 + i * 4)?);
        }
        let mut p = 4 + 4 * n + 4;
        let count = u32at(p)?;
        p += 4;
        let mut keys = Vec::with_capacity(count);
        for _ in 0..count {
            let l = u32at(p)?;
            p += 4;
            if p + l + 1 > d.len() {
                bail!("string table keys truncated");
            }
            keys.push(String::from_utf8_lossy(&d[p..p + l]).to_string());
            p += l + 1;
        }
        let mut langs = Vec::new();
        let mut seen: Vec<usize> = Vec::new();
        for &o in &offs {
            // unused languages point at the end of the file or repeat a block
            if o + 4 > d.len() || seen.contains(&o) {
                langs.push(HashMap::new());
                continue;
            }
            seen.push(o);
            let c = u32at(o)?;
            let mut q = o + 4;
            let mut m = HashMap::with_capacity(c);
            for _ in 0..c {
                if q + 10 > d.len() {
                    break;
                }
                let h = u32at(q)? as u32;
                let fl = u32at(q + 6)?;
                q += 10 + fl;
                let tl = u32at(q)?;
                q += 4;
                if q + tl * 2 + 10 > d.len() {
                    break;
                }
                let units: Vec<u16> = d[q..q + tl * 2].chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).take_while(|&u| u != 0).collect();
                m.insert(h, String::from_utf16_lossy(&units));
                q += tl * 2 + 10;
            }
            langs.push(m);
        }
        // (Portuguese: the English block with the pt-BR entries laid over it - a key the block has is replaced, one it has
        // not (the port's own rows) is added)
        if portuguese() {
            if let (Ok(g), Some(en)) = (OVERLAY.read(), langs.first_mut()) {
                if let Some(over) = g.as_ref() {
                    for (h, s) in over {
                        en.insert(*h, s.clone());
                    }
                }
            }
        }
        let mut t = StringTable { keys, langs, lang: 0 };
        t.set_lang(language());
        Ok(t)
    }

    /// Add the entries of another table (DLC and tip tables next to the main one).
    pub fn merge(&mut self, other: StringTable) {
        self.keys.extend(other.keys);
        for (i, l) in other.langs.into_iter().enumerate() {
            if i >= self.langs.len() {
                self.langs.push(l);
            } else {
                for (k, v) in l {
                    self.langs[i].entry(k).or_insert(v);
                }
            }
        }
    }

    /// The language to read: a block the table has and that holds text; anything else is
    /// English.
    pub fn set_lang(&mut self, lang: usize) {
        self.lang = if self.langs.get(lang).map_or(false, |m| !m.is_empty()) { lang } else { 0 };
    }

    /// Text of a key in the table's language, in English where that has none (keys are
    /// case-insensitive).
    pub fn get(&self, key: &str) -> Option<&str> {
        let h = joaat(key);
        self.langs.get(self.lang).and_then(|m| m.get(&h)).or_else(|| self.langs.first()?.get(&h)).map(|s| s.as_str())
    }

    pub fn get_or<'a>(&'a self, key: &'a str) -> &'a str {
        self.get(key).unwrap_or(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A one-language table with one key.
    fn table(key: &str, text: &str) -> Vec<u8> {
        let mut d: Vec<u8> = Vec::new();
        d.extend(1u32.to_le_bytes()); // one language
        d.extend(0u32.to_le_bytes()); // its offset, set below
        d.extend(1u32.to_le_bytes()); // version
        d.extend(1u32.to_le_bytes()); // key count
        d.extend((key.len() as u32).to_le_bytes());
        d.extend(key.as_bytes());
        d.push(0);
        let off = d.len() as u32;
        d[4..8].copy_from_slice(&off.to_le_bytes());
        d.extend(1u32.to_le_bytes()); // entries of the block
        d.extend(joaat(key).to_le_bytes());
        d.extend(0u16.to_le_bytes()); // flags
        d.extend(0u32.to_le_bytes()); // font name length
        let units: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        d.extend((units.len() as u32).to_le_bytes());
        for u in units {
            d.extend(u.to_le_bytes());
        }
        d.extend([0u8; 10]); // scale x, scale y, u16
        d
    }

    #[test]
    fn the_portuguese_layer_replaces_and_adds_to_the_english_block() {
        let d = table("pause_options", "OPTIONS");
        assert_eq!(StringTable::parse(&d).unwrap().get("pause_options"), Some("OPTIONS"));
        assert_eq!(set_overlay("# comment\npause_options\tOPÇÕES\nquit\tSAIR DO JOGO\\nOK\n"), 2);
        set_portuguese(true);
        let t = StringTable::parse(&d).unwrap();
        assert_eq!(t.get("pause_options"), Some("OPÇÕES"));
        assert_eq!(t.get("quit"), Some("SAIR DO JOGO\nOK"));
        set_portuguese(false);
        assert_eq!(StringTable::parse(&d).unwrap().get("pause_options"), Some("OPTIONS"));
        set_overlay("");
    }

    #[test]
    fn the_subtitle_layer_is_keyed_by_scene_and_start_time() {
        assert_eq!(set_subtitle_overlay("CUT_A\t1.5\tOlá\\ntudo bem?\ncut_b\t2\tOi\n"), 2);
        assert_eq!(subtitle_override("cut_a", "1.5").as_deref(), Some("Olá\ntudo bem?"));
        assert_eq!(subtitle_override("CUT_B", "2").as_deref(), Some("Oi"));
        assert_eq!(subtitle_override("CUT_B", "3"), None);
        set_subtitle_overlay("");
    }
}

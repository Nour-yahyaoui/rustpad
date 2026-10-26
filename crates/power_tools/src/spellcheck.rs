//! Fast Offline Spellchecker with built-in vocabulary & Levenshtein distance-1 suggestions.

use doc_model::{Block, Document};
use std::collections::HashSet;

const BUILTIN_WORDS: &str = "
a about above across action active actual add added addition address advanced after again against age
all allow almost along already also always am among amount an analysis and another answer any anyone
anything app application apply approach approved architecture area around art article as ask at author
auto automatic available average away back background balance bar base based basic be beautiful because
become before began begin behind being believe below best better between beyond big binary bit black
blank block blue board body bold book border both bottom box break brief bright bring build building
built bullet business button by byte cache call calm can canvas capable capacity card care case fast
center change chapter character check checklist choice choose clean clear click client close cloud code
color column combine command comment commercial common company compare complete compress computer
conclusion condition confidential config connect contact content contract control convert converter copy
core corner cost could count counter create created creator crisp cross current cursor custom cut dark
data date day daylight deal decide decision default define delete deliver department design desktop
detail detailed develop developer dialog diff digital direct directory disk display distance document
docx does done down download draft drag draw drawing drop due during each early easy edge edit edited
editing editor effect effort either element email embed embedded empty enable end engine engineer
ensure enter entire entry environment equal error especially essential estimate even evening event
every everyone everything exact example executive exist existing experience export extract eye face
fact fail fall family far fast feature feel field file fill filter final find fine finish first fit
five fix fixed flag flat float flow focus folder follow font footer for force form formal format
formatted foundation four frame free from front full function further future gallery gap general
generate generated geometry get github give given glare global go goal good govern grammar gray great
green grid group grow guide half hand handle happen hard hardware has have head header heading health
heavy height help her here high highlight history hit hold home horizontal host hour how however human
hundred icon idea identify if image immediate impact implement important improve in include including
income increase indent index indicate individual industry info information initial inline input insert
inside inspect inspector install installer instant instead instruction integration interface internal
into introduce invoice is issue it italic item its itself join jump just keep key keyboard kind know
knowledge label landscape language large last late launch layer layout lead learn least leave left
legal length less letter level library license life light lightweight like line link list live load
local lock log logo long look lossless low machine made main maintain major make manage manager manual
many margin mark markdown market master match material matrix max maximum may maybe me mean measure
media member memory menu merge message metadata method metric microsoft might million mind minimal
minute miss mode model modern modify module moment month more morning most move much multi multiple
must my name native natural near need network never new next nice night no node none normal not note
nothing notice now number numbered object ocr off office official offline often ok old on once one
only open openxml operate operation option optional or order ordinary organization original other our
out outline output over overall overlay own owner pack package pad page pages paint palette panel
paper paragraph parallel part partner pass password past path pattern pay payment pdf pen people per
percent performance period permanent permission person personal phone photo physical pick picture
piece pill pixel place plain plan planned planning platform play please plus point pointer polish pool
popover popular port portrait position possible power powerful practice preference prepare present
preserve press preview previous price primary print privacy private probable process processor produce
product production profile program project prompt property proposal protect provide public publish pull
pure purpose push put quality quarter question quick quiet quote radian radio rail ram range rapid
rate rather raw reach read readable reader reading ready real really reason receive recent recognize
record recover recovery redact redaction redo reduce reference region register registry regular relate
relationship release relevant reliable remain remember remove render renderer replace report represent
request require research resize resolution resource respect response restore result return reveal
review rewind ribbon rich right ring rise risk roadmap role room root rotate round row rule ruler run
running rust rustpad safe same sample save saved scale scan scanned场景 schema science scope screen
scroll search searchable second section security see seek seem select selected selection self send
sense sensitive separate sequence serif series service session set setting setup seven several shadow
shape share sheet shell shift short shortcut show showcase shrink side sign signature simple since
single site six size skill skip slate slice slide slight slow small smart smooth snapshot so soft
software solid solution some someone something sometimes soon sort source space spacing span special
specific speed spell spellcheck split spot stack stage standard start starter state status stay step
sticky still stock stop storage store storm story straight strategy stream street strength stress
strike strikethrough stroke strong structure structured studio style styled subtitle subtle success
such sudden suggest summary sun support sure surface switch symbol system tab table tag take talk
target task team technical technology tell template ten term test text than thank that the their them
theme themselves then theory there these they thing think third this those though thought three
through throughout throw thumb thumbnail thus time timeline tiny title to today together token tool
toolbar toolkit top total touch toward town track trade traditional trail train transaction transfer
transform transition transparent tree trial true truetype try turn two type typed typeface typography
ui ultra under underline undo unified unit universal unix unknown unless unlock until up update
updated updater upload upon upper us usage use used user using usual utility valid value variant
various vector verify version vertical very via view viewer viewport virtual visible visit visual
voice volume wait walk wall want warm warn warning wash watch water watermark way we wear web website
week weight welcome well were what whatever when where whether which while white who whole whom whose
why wide width wife will win window windows wing winner winter wire wireframe wish with within without
word words work worker workflow working workspace world would write writer writing wrong wrote year
yellow yes yet you young your yourself zero zip zoom zstd
";

pub struct SpellChecker {
    dict: HashSet<String>,
    sorted_list: Vec<String>,
}

impl Default for SpellChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl SpellChecker {
    pub fn new() -> Self {
        let mut dict = HashSet::new();
        for w in BUILTIN_WORDS.split_whitespace() {
            let clean: String = w
                .chars()
                .filter(|c| c.is_ascii_alphabetic())
                .collect::<String>()
                .to_ascii_lowercase();
            if !clean.is_empty() {
                dict.insert(clean);
            }
        }
        // Add author & domain terms so they never flag false positives
        for extra in ["nour", "yahyaoui", "rustpad", "openxml", "winansi", "mahdia"] {
            dict.insert(extra.to_string());
        }
        let mut sorted_list: Vec<String> = dict.iter().cloned().collect();
        sorted_list.sort();
        Self { dict, sorted_list }
    }

    pub fn add_word(&mut self, word: &str) {
        let low = word.trim().to_ascii_lowercase();
        if !low.is_empty() && self.dict.insert(low.clone()) {
            self.sorted_list.push(low);
        }
    }

    pub fn is_valid(&self, raw_word: &str) -> bool {
        let trimmed = raw_word.trim_matches(|c: char| !c.is_alphabetic());
        if trimmed.len() <= 2 {
            return true;
        }
        if trimmed.chars().any(|c| !c.is_ascii_alphabetic()) {
            return true;
        }
        // Skip ALL-CAPS acronyms (e.g., PDF, DOCX, CPU, RAM)
        if trimmed.chars().all(|c| c.is_ascii_uppercase()) {
            return true;
        }
        let low = trimmed.to_ascii_lowercase();
        if self.dict.contains(&low) {
            return true;
        }
        // Common inflections (-s, -es, -ed, -ing, -ly, -er)
        for suffix in ["s", "es", "ed", "ing", "ly", "er", "ers", "tion"] {
            if let Some(stem) = low.strip_suffix(suffix) {
                if stem.len() >= 3
                    && (self.dict.contains(stem) || self.dict.contains(&format!("{stem}e")))
                {
                    return true;
                }
            }
        }
        false
    }

    pub fn suggestions(&self, word: &str, limit: usize) -> Vec<String> {
        let target = word
            .trim_matches(|c: char| !c.is_ascii_alphabetic())
            .to_ascii_lowercase();
        if target.is_empty() {
            return Vec::new();
        }
        let mut scored: Vec<(usize, &String)> = self
            .sorted_list
            .iter()
            .filter(|cand| cand.len().abs_diff(target.len()) <= 2)
            .filter_map(|cand| {
                let d = edit_distance(&target, cand);
                if d <= 2 {
                    Some((d, cand))
                } else {
                    None
                }
            })
            .collect();
        scored.sort_by_key(|(d, w)| (*d, w.len()));
        scored
            .into_iter()
            .take(limit)
            .map(|(_, w)| w.clone())
            .collect()
    }

    /// Scans a single line or paragraph string and returns `(char_start, char_end, word)` spans.
    pub fn check_paragraph(&self, text: &str) -> Vec<(usize, usize, String)> {
        let mut out = Vec::new();
        let chars: Vec<char> = text.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            while i < chars.len() && !chars[i].is_alphabetic() {
                i += 1;
            }
            let start = i;
            while i < chars.len() && (chars[i].is_alphabetic() || chars[i] == '\'') {
                i += 1;
            }
            if i > start {
                let word: String = chars[start..i].iter().collect();
                let cleaned = word.trim_matches('\'');
                if !cleaned.is_empty() && !self.is_valid(cleaned) {
                    out.push((start, i, cleaned.to_string()));
                }
            }
        }
        out
    }

    /// Scans a `Document` and returns `(block_index, bad_word, suggestions)` entries.
    pub fn check_document(&self, doc: &Document) -> Vec<(usize, String, Vec<String>)> {
        let mut issues = Vec::new();
        for (idx, block) in doc.blocks.iter().enumerate() {
            if let Block::Paragraph(p) = block {
                for raw in p.plain_text().split_whitespace() {
                    let cleaned = raw.trim_matches(|c: char| !c.is_alphabetic());
                    if !cleaned.is_empty() && !self.is_valid(cleaned) {
                        let sugg = self.suggestions(cleaned, 3);
                        issues.push((idx, cleaned.to_string(), sugg));
                    }
                }
            }
        }
        issues
    }
}

fn edit_distance(a: &str, b: &str) -> usize {
    let ac: Vec<char> = a.chars().collect();
    let bc: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=bc.len()).collect();
    let mut curr = vec![0usize; bc.len() + 1];
    for (i, &ca) in ac.iter().enumerate() {
        curr[0] = i + 1;
        for (j, &cb) in bc.iter().enumerate() {
            let cost = usize::from(ca != cb);
            curr[j + 1] = (prev[j + 1] + 1).min(curr[j] + 1).min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[bc.len()]
}

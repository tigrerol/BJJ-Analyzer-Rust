use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tracing::{info, warn};

/// BJJ term categories for better organization
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum BJJTermCategory {
    Positions,
    Submissions,
    Techniques,
    Portuguese,
    Japanese,
    General,
}

/// BJJ dictionary for transcription enhancement
#[derive(Debug, Clone)]
pub struct BJJDictionary {
    /// Terms organized by category
    terms: HashMap<BJJTermCategory, Vec<String>>,
    
    /// Common corrections mapping (wrong -> correct)
    corrections: HashMap<String, String>,
    
    /// Prompt template for Whisper
    prompt_template: String,
}

impl BJJDictionary {
    /// Create a new BJJ dictionary with default terms
    pub fn new() -> Self {
        let mut dictionary = Self {
            terms: HashMap::new(),
            corrections: HashMap::new(),
            prompt_template: Self::default_prompt_template(),
        };
        
        dictionary.load_default_terms();
        dictionary.load_default_corrections();
        dictionary
    }
    
    /// Load dictionary from configuration file
    pub async fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = tokio::fs::read_to_string(path.as_ref()).await?;
        let mut dictionary = Self::new();
        dictionary.parse_terms_file(&content)?;
        info!("📚 Loaded BJJ dictionary from: {}", path.as_ref().display());
        Ok(dictionary)
    }
    
    /// Generate BJJ-specific prompt for Whisper
    pub fn generate_prompt(&self) -> String {
        let key_terms = self.get_key_terms_for_prompt();
        self.prompt_template.replace("{key_terms}", &key_terms.join(", "))
    }
    
    /// Get terms for a specific category
    pub fn get_terms(&self, category: &BJJTermCategory) -> Vec<String> {
        self.terms.get(category).cloned().unwrap_or_default()
    }
    
    /// Get all terms as a flat list
    pub fn get_all_terms(&self) -> Vec<String> {
        self.terms.values().flatten().cloned().collect()
    }
    
    /// Get common corrections mapping
    pub fn get_corrections(&self) -> &HashMap<String, String> {
        &self.corrections
    }
    
    /// Add a term to a category
    pub fn add_term(&mut self, category: BJJTermCategory, term: String) {
        self.terms.entry(category).or_insert_with(Vec::new).push(term);
    }
    
    /// Add a correction mapping
    pub fn add_correction(&mut self, wrong: String, correct: String) {
        self.corrections.insert(wrong.to_lowercase(), correct);
    }
    
    /// Check if a term exists in the dictionary
    pub fn contains_term(&self, term: &str) -> bool {
        let term_lower = term.to_lowercase();
        self.terms.values().flatten().any(|t| t.to_lowercase() == term_lower)
    }
    
    /// Get suggested correction for a term
    pub fn get_correction(&self, term: &str) -> Option<&String> {
        self.corrections.get(&term.to_lowercase())
    }
    
    /// Load default BJJ terms (ported from Python implementation)
    fn load_default_terms(&mut self) {
        // General & Positional Concepts
        let positions = vec![
            "Guard", "Open Guard", "Closed Guard", "Half Guard", "Deep Half Guard", 
            "Z-Guard", "Reverse Z-Guard", "K-Guard", "Knee Shield Half Guard", 
            "Butterfly Guard", "Seated Guard", "Standing Guard Pass", "Guard Retention", 
            "Guard Recovery", "Guard Pull", "Turtle", "Sprawl", "Base", "Posture", 
            "Framing", "Grips", "Grip Fighting", "Collar Grip", "Sleeve Grip", 
            "Pistol Grip", "C-Grip", "Monkey Grip", "Gable Grip", "S-Grip", 
            "Seatbelt Grip", "Underhook", "Overhook", "Whizzer", "Crossface", 
            "Connection", "Pressure", "Distance Management", "Angles", "Level Change", 
            "Hip Escape", "Shrimp", "Bridge", "Upa", "Granby Roll", "Technical Stand-up", 
            "Posting", "Clinch", "Pummeling", "Mount", "High Mount", "Low Mount", 
            "S-Mount", "Technical Mount", "Back Mount", "Back Control", "Hooks", 
            "Body Triangle", "Side Control", "Kesa Gatame", "Reverse Kesa Gatame", 
            "North-South", "Knee on Belly", "Twister Side Control", "Headquarters", "HQ",
            "Leg Drag", "Stack Pass", "Torreando Pass", "Over-Under Pass", 
            "Double Under Pass", "Knee Cut Pass", "Knee Slice Pass", "X-Pass", 
            "Smash Pass", "Pressure Pass", "Folding Pass", "Forced Half Guard Pass",
            "Spider Guard", "Lasso Guard", "Spider-Lasso Hybrid", "De La Riva Guard", 
            "Reverse De La Riva Guard", "X-Guard", "Single Leg X-Guard", "50/50 Guard", 
            "Ashi Garami", "Outside Ashi Garami", "Inside Sankaku", "Saddle", 
            "Honey Hole", "411", "Lockdown", "Electric Chair", "Donkey Guard", 
            "Rubber Guard", "Williams Guard", "Crab Ride", "Leg Pummeling",
            "Berimbolo", "Inversion", "Inverted"
        ];
        
        // Submissions
        let submissions = vec![
            "Armbar", "Straight Armbar", "Juji Gatame", "Kimura", "Americana", 
            "Keylock", "Ude Garami", "Omoplata", "Sankaku Garami", "Wrist Lock", 
            "Gogoplata", "Triangle Choke", "Arm Triangle Choke", "D'Arce Choke", 
            "Brabo Choke", "Anaconda Choke", "Peruvian Necktie", "Japanese Necktie", 
            "Guillotine Choke", "High Elbow Guillotine", "Arm-in Guillotine", 
            "Rear Naked Choke", "RNC", "Mata Leão", "Bow and Arrow Choke", 
            "Cross Collar Choke", "Sliding Collar Choke", "Ezekiel Choke", 
            "Sode Guruma Jime", "Clock Choke", "Paper Cutter Choke", 
            "Baseball Bat Choke", "Loop Choke", "North-South Choke", 
            "Von Flue Choke", "Heel Hook", "Inside Heel Hook", "Outside Heel Hook", 
            "Kneebar", "Toe Hold", "Ankle Lock", "Straight Ankle Lock", 
            "Achilles Lock", "Estima Lock", "Calf Crank", "Calf Slicer", 
            "Bicep Slicer", "Tarikoplata", "Baratoplata", "Monoplata", "Mir Lock",
            "Groin Stretch", "Banana Split", "Crotch Ripper"
        ];
        
        // Techniques
        let techniques = vec![
            "Submission", "Tap", "Tap Out", "Roll", "Spar", "Flow Roll", "Drill", 
            "Positional Sparring", "Specific Training", "Live Training", "Scramble", 
            "Transition", "Reversal", "Sweep", "Counter", "Defense", "Escape", 
            "Pin", "Dominant Position", "Neutral Position", "Bad Position", 
            "Top Position", "Bottom Position", "Inside Position", "Outside Position", 
            "Leverage", "Momentum", "Timing", "Misdirection", "Feint", "Combination",
            "Body Lock", "Body Lock Pass", "Wrestling Up", "Wall Walking", 
            "Take Down", "Single Leg Takedown", "Double Leg Takedown", 
            "Ankle Pick", "Snap Down", "Arm Drag", "Foot Sweep", "Throw", 
            "Hip Toss", "Off-balancing", "Kuzushi"
        ];
        
        // Portuguese Terms
        let portuguese = vec![
            "Guarda", "Guarda Fechada", "Meia Guarda", "Guarda Aranha", 
            "Guarda De La Riva", "Guarda X", "Guarda Borboleta", "Montada", 
            "Cem Quilos", "Joelho na Barriga", "Pegada nas Costas", "Baiana", 
            "Queda", "Raspagem", "Passagem de Guarda", "Estrangulamento", 
            "Mata Leão", "Triângulo", "Americana", "Kimura", "Omoplata", 
            "Chave de Braço", "Chave de Pé", "Chave de Calcanhar", "Chave de Joelho", 
            "Pegada", "Gola", "Manga", "Lapela", "Calça", "Faixa", "Kimono", 
            "Rolamento", "Fuga de Quadril", "Ponte", "Upa", "Berimbolo", 
            "Leg Drag", "Rodado", "Passador", "Guardeiro", "Treino", "Amasso", 
            "Bater", "Deixa rolar", "Puxar para a guarda", "Quedar", "Postura", 
            "Base", "Tatame", "Luta", "Jiu-Jitsu", "Capotagem", "Canelada", 
            "Esgrima", "Cadeado", "Crucifixo", "Mão de Vaca", "Gravata"
        ];
        
        // Japanese Terms
        let japanese = vec![
            "Jiu-Jitsu", "Dojo", "Sensei", "Professor", "Oss", "Gi", "Kimono", 
            "Obi", "Shime", "Jime", "Newaza", "Tachi-waza", "Ukemi", "Nage-waza", 
            "Kesa Gatame", "Kuzure Kesa Gatame", "Ushiro Kesa Gatame", "Kata Gatame", 
            "Juji Gatame", "Ude Garami", "Ude Hishigi", "Ashi Garami", "Hiza Gatame", 
            "Sankaku Jime", "Hadaka Jime", "Okuri Eri Jime", "Kataha Jime", 
            "Sode Guruma Jime", "Tatami", "Randori", "Shiai", "Hajime", "Matte", 
            "Sore made", "Rei", "Kuzushi"
        ];
        
        // Store terms by category
        self.terms.insert(BJJTermCategory::Positions, positions.into_iter().map(String::from).collect());
        self.terms.insert(BJJTermCategory::Submissions, submissions.into_iter().map(String::from).collect());
        self.terms.insert(BJJTermCategory::Techniques, techniques.into_iter().map(String::from).collect());
        self.terms.insert(BJJTermCategory::Portuguese, portuguese.into_iter().map(String::from).collect());
        self.terms.insert(BJJTermCategory::Japanese, japanese.into_iter().map(String::from).collect());
    }
    
    /// Load common corrections (wrong -> correct)
    fn load_default_corrections(&mut self) {
        let corrections = vec![
            ("coast guard", "closed guard"),
            ("clothes guard", "closed guard"),
            ("close guard", "closed guard"),
            ("butterfly god", "butterfly guard"),
            ("spider god", "spider guard"),
            ("x god", "x guard"),
            ("arm bar", "armbar"),
            ("kimora", "kimura"),
            ("america", "americana"),
            ("americana", "americana"),
            ("triangle joke", "triangle choke"),
            ("rear naked joke", "rear naked choke"),
            ("grappling", "grappling"),
            ("jujitsu", "jiu-jitsu"),
            ("jujitsu", "jiu jitsu"),
            ("jiu jitsu", "jiu-jitsu"),
            ("gee", "gi"),
            ("no gee", "no-gi"),
            ("no gi", "no-gi"),
            ("matt", "mat"),
            ("mats", "mats"),
            ("tatami", "tatami"),
            ("side mount", "side control"),
            ("north south", "north-south"),
            ("knee on belly", "knee on belly"),
            ("scarfhold", "scarf hold"),
            ("key lock", "keylock"),
            ("heel hook", "heel hook"),
            ("kneebar", "kneebar"),
            ("toe hold", "toe hold"),
            ("ankle lock", "ankle lock"),
            ("calf slicer", "calf slicer"),
            ("bicep slicer", "bicep slicer"),
            ("de la riva", "de la riva"),
            ("delaware", "de la riva"),
            ("dela riva", "de la riva"),
            ("reverse dela riva", "reverse de la riva"),
            ("berimbolo", "berimbolo"),
            ("berimbolos", "berimbolos"),
            ("leg drag", "leg drag"),
            ("torreando", "torreando"),
            ("tornado", "torreando"),
            ("bullfighter", "torreando"),
            ("knee cut", "knee cut"),
            ("knee slice", "knee slice"),
            ("smash pass", "smash pass"),
            ("pressure pass", "pressure pass"),
            ("stack pass", "stack pass"),
            ("headquarters", "headquarters"),
            ("hq", "headquarters"),
            ("ashi garami", "ashi garami"),
            ("outside ashi", "outside ashi garami"),
            ("inside sankaku", "inside sankaku"),
            ("50 50", "50/50"),
            ("fiftyfifty", "50/50"),
            ("fifty fifty", "50/50"),
            ("honey hole", "honey hole"),
            ("saddle", "saddle"),
            ("411", "411"),
            ("four eleven", "411"),
            ("lockdown", "lockdown"),
            ("electric chair", "electric chair"),
            ("rubber guard", "rubber guard"),
            ("mission control", "mission control"),
            ("williams guard", "williams guard"),
            ("donkey guard", "donkey guard"),
            ("crab ride", "crab ride"),
            ("truck", "truck"),
            ("twister", "twister"),
            ("banana split", "banana split"),
            ("crotch ripper", "crotch ripper"),
            ("oil check", "oil check"),
        ];
        
        for (wrong, correct) in corrections {
            self.corrections.insert(wrong.to_string(), correct.to_string());
        }
    }
    
    /// Get key terms for prompt generation (most important terms)
    fn get_key_terms_for_prompt(&self) -> Vec<String> {
        let mut key_terms = Vec::new();
        
        // Add most common positions
        key_terms.extend_from_slice(&[
            "guard", "mount", "side control", "back control", "half guard",
            "closed guard", "open guard", "butterfly guard", "spider guard"
        ]);
        
        // Add most common submissions
        key_terms.extend_from_slice(&[
            "armbar", "triangle", "kimura", "americana", "rear naked choke",
            "guillotine", "omoplata", "heel hook", "ankle lock"
        ]);
        
        // Add common techniques
        key_terms.extend_from_slice(&[
            "sweep", "escape", "pass", "submission", "takedown", "transition"
        ]);
        
        key_terms.into_iter().map(String::from).collect()
    }
    
    /// Parse terms from configuration file
    fn parse_terms_file(&mut self, content: &str) -> Result<()> {
        let mut current_category = BJJTermCategory::General;
        
        for line in content.lines() {
            let line = line.trim();
            
            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            
            // Check for category headers
            if line.starts_with('[') && line.ends_with(']') {
                let category_name = &line[1..line.len()-1];
                current_category = match category_name.to_lowercase().as_str() {
                    "positions" => BJJTermCategory::Positions,
                    "submissions" => BJJTermCategory::Submissions,
                    "techniques" => BJJTermCategory::Techniques,
                    "portuguese" => BJJTermCategory::Portuguese,
                    "japanese" => BJJTermCategory::Japanese,
                    _ => BJJTermCategory::General,
                };
                continue;
            }
            
            // Check for corrections (format: wrong -> correct)
            if line.contains(" -> ") {
                let parts: Vec<&str> = line.split(" -> ").collect();
                if parts.len() == 2 {
                    self.add_correction(parts[0].trim().to_string(), parts[1].trim().to_string());
                }
                continue;
            }
            
            // Add term to current category
            self.add_term(current_category.clone(), line.to_string());
        }
        
        Ok(())
    }
    
    /// Default prompt template for Whisper
    fn default_prompt_template() -> String {
        "Brazilian Jiu-Jitsu instructional video featuring techniques including {key_terms}. \
         The speaker will discuss positions, submissions, grips, and escapes using BJJ terminology.".to_string()
    }
    
    /// Get statistics about the dictionary
    pub fn get_stats(&self) -> BJJDictionaryStats {
        let total_terms = self.terms.values().map(|v| v.len()).sum();
        let total_corrections = self.corrections.len();
        
        let category_counts: HashMap<BJJTermCategory, usize> = self.terms
            .iter()
            .map(|(k, v)| (k.clone(), v.len()))
            .collect();
        
        BJJDictionaryStats {
            total_terms,
            total_corrections,
            category_counts,
        }
    }
}

impl Default for BJJDictionary {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about the BJJ dictionary
#[derive(Debug, Clone)]
pub struct BJJDictionaryStats {
    pub total_terms: usize,
    pub total_corrections: usize,
    pub category_counts: HashMap<BJJTermCategory, usize>,
}

impl BJJDictionaryStats {
    /// Generate a summary string
    pub fn summary(&self) -> String {
        format!(
            "BJJ Dictionary Stats:\n\
            - Total terms: {}\n\
            - Total corrections: {}\n\
            - Categories: {:?}",
            self.total_terms,
            self.total_corrections,
            self.category_counts
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bjj_dictionary_creation() {
        let dict = BJJDictionary::new();
        let stats = dict.get_stats();
        
        assert!(stats.total_terms > 0);
        assert!(stats.total_corrections > 0);
        assert!(dict.contains_term("guard"));
        assert!(dict.contains_term("armbar"));
    }

    #[test]
    fn test_prompt_generation() {
        let dict = BJJDictionary::new();
        let prompt = dict.generate_prompt();
        
        assert!(prompt.contains("Brazilian Jiu-Jitsu"));
        assert!(prompt.contains("guard"));
        assert!(prompt.contains("mount"));
    }

    #[test]
    fn test_corrections() {
        let dict = BJJDictionary::new();
        
        assert_eq!(dict.get_correction("coast guard"), Some(&"closed guard".to_string()));
        assert_eq!(dict.get_correction("arm bar"), Some(&"armbar".to_string()));
        assert_eq!(dict.get_correction("nonexistent"), None);
    }

    #[test]
    fn test_category_access() {
        let dict = BJJDictionary::new();
        
        let positions = dict.get_terms(&BJJTermCategory::Positions);
        let submissions = dict.get_terms(&BJJTermCategory::Submissions);
        
        assert!(!positions.is_empty());
        assert!(!submissions.is_empty());
        assert!(positions.iter().any(|p| p.contains("Guard")));
        assert!(submissions.iter().any(|s| s.contains("Armbar")));
    }
}
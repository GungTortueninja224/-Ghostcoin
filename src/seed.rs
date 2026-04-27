use sha2::{Sha256, Digest};
use rand::RngCore;
use rand::rngs::OsRng;

// 200 mots simples pour la seed phrase
const WORDLIST: &[&str] = &[
    "apple","banana","cherry","dragon","eagle","forest","garden","harbor",
    "island","jungle","kernel","lemon","mango","night","ocean","planet",
    "queen","river","silver","tiger","ultra","violet","winter","xenon",
    "yellow","zebra","alpha","bravo","charlie","delta","echo","foxtrot",
    "ghost","hotel","india","juliet","kilo","lima","mike","november",
    "oscar","papa","quebec","romeo","sierra","tango","uniform","victor",
    "whisky","xray","yankee","zulu","anchor","bridge","castle","diamond",
    "energy","flash","golden","hunter","impact","jewel","knight","laser",
    "matrix","nebula","origin","portal","quartz","rocket","shadow","thunder",
    "unique","valley","wonder","xenos","youth","zenith","arctic","beacon",
    "cosmic","drift","ember","flame","glacier","haven","inferno","jasper",
    "karma","lunar","mystic","nexus","oracle","prism","quantum","raven",
    "storm","titan","umbra","vortex","wave","xylem","yarn","zephyr",
    "abyss","blaze","cipher","dusk","eclipse","frost","glyph","horizon",
    "iris","jade","kraken","lava","mirage","nova","onyx","peak",
    "quest","ridge","sphinx","tundra","umber","vale","wisp","xeric",
    "yonder","zone","atom","bolt","crane","dawn","elder","fern",
    "grove","haze","icon","jewel","knoll","leaf","marsh","nook",
    "opal","pine","quill","reed","slate","trail","urn","vine",
    "wool","xylo","yew","zinc","amber","brook","cliff","dale",
    "elm","ford","glen","hill","isle","kite","lake","mist",
    "nile","oak","pond","quay","reef","sand","tide","ursa",
    "vale","wake","xyst","yard","zeal",
];

pub struct SeedPhrase {
    pub words:   Vec<String>,
    pub entropy: Vec<u8>,
}

impl SeedPhrase {
    // Génère une seed phrase de 12 mots
    pub fn generate() -> Self {
        let mut rng     = OsRng;
        let mut entropy = vec![0u8; 16];
        rng.fill_bytes(&mut entropy);

        let words = Self::entropy_to_words(&entropy);

        Self { words, entropy }
    }

    // Restaure depuis les mots
    pub fn from_words(words: &[&str]) -> Option<Self> {
        if words.len() != 12 {
            println!("❌ Il faut exactement 12 mots");
            return None;
        }

        // Vérifie que tous les mots sont valides
        for word in words {
            if !WORDLIST.contains(word) {
                println!("❌ Mot invalide : {}", word);
                return None;
            }
        }

        let entropy = Self::words_to_entropy(words)?;

        Some(Self {
            words:   words.iter().map(|w| w.to_string()).collect(),
            entropy,
        })
    }

    // Convertit entropy → mots
    fn entropy_to_words(entropy: &[u8]) -> Vec<String> {
        let mut words = Vec::new();
        for i in 0..12 {
            let idx = entropy[i % entropy.len()] as usize % WORDLIST.len();
            words.push(WORDLIST[idx].to_string());
        }
        words
    }

    // Convertit mots → entropy
    fn words_to_entropy(words: &[&str]) -> Option<Vec<u8>> {
        let mut entropy = Vec::new();
        for word in words {
            let idx = WORDLIST.iter().position(|&w| w == *word)?;
            entropy.push(idx as u8);
        }
        Some(entropy)
    }

    // Dérive une clé privée depuis la seed
    pub fn to_private_key(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(&self.entropy);
        hasher.update(b"ghostcoin_key_derivation_v1");
        hex::encode(hasher.finalize())
    }

    pub fn display(&self) {
        println!("\n╔══════════════════════════════════════════════════════════╗");
        println!("║           🌱 SEED PHRASE — 12 MOTS                      ║");
        println!("╠══════════════════════════════════════════════════════════╣");
        println!("║  ⚠️  ÉCRIS CES MOTS SUR PAPIER — NE LES PARTAGE PAS    ║");
        println!("╠══════════════════════════════════════════════════════════╣");
        for (i, word) in self.words.iter().enumerate() {
            println!("║  {:>2}. {:<54} ║", i + 1, word);
        }
        println!("╠══════════════════════════════════════════════════════════╣");
        println!("║  Ces mots permettent de restaurer ton wallet             ║");
        println!("║  sur n'importe quel appareil.                            ║");
        println!("╚══════════════════════════════════════════════════════════╝");
    }
}
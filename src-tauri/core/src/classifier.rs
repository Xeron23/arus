//! Lightweight on-device text classifier for Indonesian personal finance transactions.
//! Uses a combination of:
//! 1. User learned memory (exact/substring overrides from user corrections in SQLite)
//! 2. Priority rules (preventing ambiguous tokens like "XXI in Mall" from being classified as "shop")
//! 3. Pre-trained Multinomial Naive Bayes trained on Indonesian merchant phrases.

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;
use regex::Regex;

/// Indonesian retail training phrases mapped to categories:
/// food, shop, transport, home, fun
const TRAINING_DATA: &[(&str, &str)] = &[
    // FOOD (105+ entries)
    ("warteg bahari", "food"), ("warteg kharisma", "food"), ("warteg agung", "food"), ("warteg barokah", "food"),
    ("warteg", "food"), ("nasi padang sederhana", "food"), ("rumah makan padang", "food"), ("rm padang", "food"),
    ("sate padang ajo ramon", "food"), ("sate madura pak kumis", "food"), ("soto ayam lamongan", "food"),
    ("soto betawi bang mamat", "food"), ("soto mie bogor", "food"), ("soto kudus", "food"), ("soto", "food"),
    ("bakmi gm", "food"), ("bakmi alok", "food"), ("bakmi roxy", "food"), ("mie ayam banyumas", "food"),
    ("mie gacoan", "food"), ("gacoan", "food"), ("bubur ayam cianjur", "food"), ("ketoprak ciragil", "food"),
    ("siomay batagor bandung", "food"), ("batagor", "food"), ("martabak san francisco", "food"),
    ("martabak manis bangka", "food"), ("martabak telur orins", "food"), ("martabak", "food"),
    ("ayam geprek bensu", "food"), ("ayam geprek juara", "food"), ("ayam bakar", "food"), ("ayam goreng pemuda", "food"),
    ("bebek kaleyo", "food"), ("bebek sinjay", "food"), ("pecel lele tenda", "food"), ("angkringan jogja", "food"),
    ("warkop kuningan", "food"), ("warkop", "food"), ("roti bakar eddi", "food"), ("gorengan", "food"),
    ("dapur solo", "food"), ("dapur sunda", "food"), ("ikan bakar cianjur", "food"), ("gudeg yu djum", "food"),
    ("nasi uduk kebon kacang", "food"), ("nasi goreng", "food"), ("kuliner", "food"),
    // Cafe, Kopi, Minuman
    ("kopi kenangan", "food"), ("fore coffee", "food"), ("janji jiwa", "food"), ("point coffee", "food"),
    ("tomoro coffee", "food"), ("starbucks coffee", "food"), ("tanamera coffee", "food"), ("excelso", "food"),
    ("djournal coffee", "food"), ("anomali coffee", "food"), ("kopi", "food"), ("coffee", "food"), ("cafe", "food"),
    ("haus indonesia", "food"), ("haus", "food"), ("chatime", "food"), ("teh tong tji", "food"),
    ("menantea", "food"), ("teh solo", "food"), ("es teh indonesia", "food"), ("es teh", "food"),
    ("boba", "food"), ("xing fu tang", "food"), ("tiger sugar", "food"), ("koi the", "food"),
    ("dum dum thai tea", "food"), ("dapur cokelat", "food"), ("jco donuts & coffee", "food"), ("jco", "food"),
    ("dunkin donuts", "food"), ("roti o", "food"), ("rotiboy", "food"), ("breadtalk", "food"),
    ("mako bakery", "food"), ("holland bakery", "food"), ("the harvest cake", "food"), ("mixue ice cream", "food"),
    ("mixue", "food"), ("jus buah segar", "food"), ("juice", "food"),
    // Restoran & Fast Food
    ("mcdonalds", "food"), ("mcd", "food"), ("kfc", "food"), ("burger king", "food"),
    ("a&w restaurant", "food"), ("wendys", "food"), ("subway", "food"), ("pizza hut", "food"),
    ("dominos pizza", "food"), ("marugame udon", "food"), ("udon", "food"), ("hokben hoka hoka bento", "food"),
    ("hokben", "food"), ("yoshinoya", "food"), ("pepper lunch", "food"), ("solaria", "food"),
    ("ichiban sushi", "food"), ("sushi tei", "food"), ("genki sushi", "food"), ("sushi", "food"),
    ("ramen ya", "food"), ("ramen", "food"), ("shaburi & kintan buffet", "food"), ("hanamasa", "food"),
    ("gyu-kaku japanese bbq", "food"), ("dcrepes", "food"), ("shihlin taiwan", "food"), ("steak 21", "food"),
    ("abuba steak", "food"), ("holy cow steak", "food"), ("barapi meat & grill", "food"), ("steak", "food"),
    ("ta wan restaurant", "food"), ("imperial kitchen", "food"), ("duck king", "food"), ("dimsum", "food"),
    ("restoran", "food"), ("resto", "food"), ("makan", "food"),

    // SHOP (90+ entries)
    ("indomaret", "shop"), ("indomaret point", "shop"), ("alfamart", "shop"), ("alfamidi", "shop"),
    ("family mart", "shop"), ("lawson station", "shop"), ("circle k", "shop"), ("superindo", "shop"),
    ("transmart", "shop"), ("hypermart", "shop"), ("lotte mart", "shop"), ("hero supermarket", "shop"),
    ("grand lucky", "shop"), ("ranch market", "shop"), ("farmers market", "shop"), ("papaya fresh gallery", "shop"),
    ("the foodhall", "shop"), ("tip top supermarket", "shop"), ("hari hari swalayan", "shop"),
    ("naga swalayan", "shop"), ("toserba yogya", "shop"), ("griya toserba", "shop"),
    ("minimarket", "shop"), ("supermarket", "shop"), ("swalayan", "shop"),
    // E-Commerce
    ("tokopedia", "shop"), ("shopee pay", "shop"), ("shopee marketplace", "shop"), ("shopee", "shop"),
    ("lazada indonesia", "shop"), ("blibli", "shop"), ("bukalapak", "shop"), ("tiktok shop", "shop"),
    ("zalora", "shop"), ("bhinneka", "shop"), ("jakmall", "shop"),
    // Retail, Pakaian, Dept Store
    ("uniqlo", "shop"), ("h&m", "shop"), ("zara", "shop"), ("pull&bear", "shop"),
    ("bershka", "shop"), ("stradivarius", "shop"), ("cotton on", "shop"), ("matahari department store", "shop"),
    ("ramayana", "shop"), ("sogo", "shop"), ("seibu", "shop"), ("metro department store", "shop"),
    ("erigo official", "shop"), ("eiger adventure store", "shop"), ("consina", "shop"), ("bodypack", "shop"),
    ("sport station", "shop"), ("foot locker", "shop"), ("planet sports", "shop"), ("bata shoes", "shop"),
    ("payless shoe source", "shop"), ("fashion", "shop"), ("baju", "shop"), ("distro", "shop"),
    ("watsons", "shop"), ("guardian apotek", "shop"), ("sociolla store", "shop"), ("the body shop", "shop"),
    ("sephora", "shop"), ("kimia farma", "shop"), ("k24 apotek", "shop"), ("century pharma", "shop"),
    ("apotek", "shop"), ("optik melawai", "shop"), ("optik seis", "shop"), ("optik tunggal", "shop"),
    ("ace hardware", "shop"), ("informa furnishings", "shop"), ("ikea", "shop"), ("mitra10", "shop"),
    ("depo bangunan", "shop"), ("erafone", "shop"), ("ibox apple reseller", "shop"), ("digimap", "shop"),
    ("xiaomi store", "shop"), ("samsung store", "shop"), ("gadget", "shop"), ("elektronik", "shop"),
    ("gramedia", "shop"), ("gunung agung", "shop"), ("miniso", "shop"), ("mr diy", "shop"),
    ("kkv", "shop"), ("flying tiger", "shop"), ("toko kelontong berkah", "shop"), ("toko sembako rejeki", "shop"),
    ("warung madura sembako", "shop"), ("toko", "shop"), ("store", "shop"),

    // TRANSPORT (50+ entries)
    ("spbu pertamina", "transport"), ("shell gas station", "transport"), ("bp akr spbu", "transport"),
    ("vivo spbu", "transport"), ("pertamina retail", "transport"), ("pom bensin pasteur", "transport"),
    ("beli bensin pertalite", "transport"), ("pertamax turbo", "transport"), ("spbu", "transport"),
    ("pertamina", "transport"), ("shell", "transport"), ("bensin", "transport"), ("solar", "transport"),
    ("pertalite", "transport"), ("pertamax", "transport"),
    // Ride-hailing & Taksi
    ("grab transport", "transport"), ("grab ride", "transport"), ("grab car", "transport"),
    ("gojek goride", "transport"), ("gojek gocar", "transport"), ("grab", "transport"),
    ("gojek", "transport"), ("goride", "transport"), ("gocar", "transport"), ("maxim transport", "transport"),
    ("indrive trip", "transport"), ("blue bird taksi", "transport"), ("silver bird", "transport"),
    ("maxim", "transport"), ("indrive", "transport"), ("taksi", "transport"), ("taxi", "transport"),
    ("ojek", "transport"),
    // Kereta, Bus & Pesawat
    ("pt kci tiket krl", "transport"), ("commuter line jabodetabek", "transport"), ("pt kai kereta api", "transport"),
    ("mrt jakarta", "transport"), ("lrt jakarta", "transport"), ("transjakarta tije", "transport"),
    ("krl", "transport"), ("mrt", "transport"), ("lrt", "transport"), ("kai", "transport"),
    ("kereta", "transport"), ("transjakarta", "transport"), ("jaklingko kartu", "transport"),
    ("damri bandara", "transport"), ("tiket garuda indonesia", "transport"), ("lion air booking", "transport"),
    ("citilink ticket", "transport"), ("airasia booking", "transport"),
    // Parkir & Tol
    ("secure parking mall", "transport"), ("centrepark", "transport"), ("sky parking", "transport"),
    ("parkir meter menteng", "transport"), ("parkir", "transport"), ("parking", "transport"),
    ("jasamarga tol purbaleunyi", "transport"), ("citra marga nusaphala tol", "transport"),
    ("astrainfra toll road", "transport"), ("tol", "transport"), ("toll", "transport"),
    // Bengkel & Cuci
    ("bengkel ahass honda", "transport"), ("bengkel resmi yamaha", "transport"),
    ("bengkel mobil auto2000", "transport"), ("nasmoco toyota", "transport"), ("bengkel", "transport"),
    ("ahass", "transport"), ("servis motor", "transport"), ("service mobil", "transport"),
    ("shop&drive", "transport"), ("bengkel bos", "transport"), ("tambal ban tubeless", "transport"),
    ("planet ban motor", "transport"), ("tambal ban", "transport"), ("ban motor", "transport"),
    ("cuci steam motor barokah", "transport"), ("autoglaze cuci mobil", "transport"),
    ("car wash poles mobil", "transport"), ("cuci motor", "transport"), ("cuci mobil", "transport"),

    // HOME (35+ entries)
    ("pln prabayar token listrik", "home"), ("pln pascabayar tagihan", "home"), ("pln", "home"),
    ("listrik", "home"), ("token listrik", "home"), ("pdam tirtawening air", "home"),
    ("pdam jaya jakarta", "home"), ("pdam", "home"), ("tagihan air", "home"),
    ("depot isi ulang air galon", "home"), ("toko gas lpg elpiji 3kg", "home"), ("galon", "home"),
    ("aqua galon", "home"), ("gas lpg", "home"), ("elpiji", "home"),
    ("indihome telkom", "home"), ("biznet home internet", "home"), ("first media cable", "home"),
    ("myrepublic wifi", "home"), ("cbn fiber internet", "home"), ("oxygen fiber", "home"),
    ("xl home fiber", "home"), ("mnc play media", "home"), ("wifi", "home"), ("internet", "home"),
    ("indihome", "home"), ("biznet", "home"), ("first media", "home"), ("myrepublic", "home"),
    ("telkom", "home"), ("pulsa telkomsel simpati", "home"), ("paket data indosat im3", "home"),
    ("xl axiata pulsa", "home"), ("smartfren paket data", "home"), ("pulsa", "home"), ("kuota", "home"),
    ("paket data", "home"), ("telkomsel", "home"), ("indosat", "home"), ("smartfren", "home"),
    ("laundry box", "home"), ("superwash laundry", "home"), ("cuci koin express", "home"),
    ("laundry kiloan wangi", "home"), ("laundry", "home"), ("cuci baju", "home"),
    ("dry cleaning baju jas", "home"), ("sewa kos kostan bu hj", "home"), ("bayar kontrakan rumah", "home"),
    ("kost", "home"), ("kosan", "home"), ("kontrakan", "home"), ("sewa rumah", "home"),
    ("ipl apartemen maintenance", "home"), ("iuran warga rt rw", "home"),
    ("iuran sampah kebersihan lingkungan", "home"), ("ipl", "home"), ("kebersihan", "home"),

    // FUN (40+ entries)
    ("cinema 21", "fun"), ("cinema xxi", "fun"), ("cgv cinemas", "fun"), ("cinepolis", "fun"),
    ("imax gandaria", "fun"), ("flix cinema", "fun"), ("xxi", "fun"), ("cgv", "fun"),
    ("cinepolis", "fun"), ("bioskop", "fun"), ("cinema", "fun"),
    ("netflix streaming subscription", "fun"), ("disney plus hotstar", "fun"),
    ("youtube premium", "fun"), ("hbo go", "fun"), ("vidio dot com", "fun"),
    ("netflix", "fun"), ("disney", "fun"), ("youtube premium", "fun"),
    ("steam games valve", "fun"), ("playstation network psn", "fun"), ("nintendo eshop", "fun"),
    ("xbox game pass", "fun"), ("steam games", "fun"), ("playstation", "fun"), ("nintendo", "fun"),
    ("game", "fun"), ("gaming", "fun"), ("codashop mobile legends", "fun"),
    ("unipin voucher game", "fun"), ("garena indonesia", "fun"), ("itemku gaming", "fun"),
    ("codashop", "fun"), ("unipin", "fun"), ("topup game", "fun"), ("diamond mlbb", "fun"),
    ("spotify premium", "fun"), ("apple music", "fun"), ("joox vip", "fun"), ("spotify", "fun"),
    ("masterpiece family karaoke", "fun"), ("happy puppy karaoke", "fun"), ("inul vizta karaoke", "fun"),
    ("karaoke", "fun"), ("galaxy billiard club", "fun"), ("bowling arena", "fun"), ("billiard", "fun"),
    ("dufan ancol tiket", "fun"), ("taman safari bogor", "fun"), ("waterbom bali", "fun"),
    ("trans studio tiket", "fun"), ("rekreasi", "fun"), ("wisata", "fun"),
    ("celebrity fitness club", "fun"), ("fitness first", "fun"), ("osbond gym", "fun"),
    ("anytime fitness", "fun"), ("gym", "fun"), ("fitness", "fun"),
    ("sewa lapangan futsal", "fun"), ("sewa lapangan badminton", "fun"),
    ("padel court jakarta", "fun"), ("tiket konser musik", "fun"), ("futsal", "fun"),
    ("badminton", "fun"), ("konser", "fun"),
];

/// Pre-compiled Naive Bayes model structure
pub struct NaiveBayesModel {
    pub class_doc_counts: HashMap<&'static str, usize>,
    pub word_counts: HashMap<&'static str, HashMap<&'static str, usize>>,
    pub class_totals: HashMap<&'static str, usize>,
    pub vocab: HashSet<&'static str>,
    pub total_docs: usize,
}

const STOP_WORDS: &[&str] = &[
    "pt", "cv", "ud", "tbk", "indonesia", "indo", "dan", "di", "ke", "dari", "yang", "untuk",
    "jaya", "abadi", "bersama", "makmur", "sentosa", "utama", "sejahtera", "lestari"
];

impl NaiveBayesModel {
    pub fn build() -> Self {
        let mut class_doc_counts = HashMap::new();
        let mut word_counts: HashMap<&'static str, HashMap<&'static str, usize>> = HashMap::new();
        let mut class_totals = HashMap::new();
        let mut vocab = HashSet::new();

        let token_regex = Regex::new(r"[a-z0-9]+").unwrap();

        for &(phrase, cat) in TRAINING_DATA {
            *class_doc_counts.entry(cat).or_insert(0) += 1;
            for mat in token_regex.find_iter(phrase) {
                let token = mat.as_str();
                if token.len() > 1 && !STOP_WORDS.contains(&token) {
                    vocab.insert(token);
                    *word_counts.entry(cat).or_default().entry(token).or_insert(0) += 1;
                    *class_totals.entry(cat).or_insert(0) += 1;
                }
            }
        }

        let total_docs = class_doc_counts.values().sum();
        Self {
            class_doc_counts,
            word_counts,
            class_totals,
            vocab,
            total_docs,
        }
    }

    pub fn predict(&self, text: &str) -> Option<String> {
        let clean = text.to_lowercase();
        let token_regex = Regex::new(r"[a-z0-9]+").ok()?;
        let tokens: Vec<_> = token_regex
            .find_iter(&clean)
            .map(|m| m.as_str())
            .filter(|w| w.len() > 1 && !STOP_WORDS.contains(w))
            .collect();

        if tokens.is_empty() {
            return None;
        }

        let vocab_len = self.vocab.len() as f64;
        let mut scores = HashMap::new();

        for (&cat, &doc_cnt) in &self.class_doc_counts {
            let prior = ((doc_cnt as f64) / (self.total_docs as f64)).ln();
            let denom = (*self.class_totals.get(cat).unwrap_or(&0) as f64) + vocab_len;
            let mut score = prior;
            let mut matched = 0;

            for &t in &tokens {
                let count = self
                    .word_counts
                    .get(cat)
                    .and_then(|w| w.get(t))
                    .copied()
                    .unwrap_or(0);
                if count > 0 {
                    matched += 1;
                }
                score += (((count + 1) as f64) / denom).ln();
            }

            if matched > 0 {
                scores.insert(cat, score);
            }
        }

        scores
            .into_iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(cat, _)| cat.to_string())
    }
}

static MODEL: LazyLock<NaiveBayesModel> = LazyLock::new(NaiveBayesModel::build);

static HIGH_PRIORITY_RULES: LazyLock<Vec<(Regex, &'static str)>> = LazyLock::new(|| {
    vec![
        (Regex::new(r"(?i)\b(?:xxi|cgv|cinepolis|bioskop|cinema)\b").unwrap(), "fun"),
        (Regex::new(r"(?i)\b(?:laundry|cuci\s*koin|cuci\s*kiloan)\b").unwrap(), "home"),
        (Regex::new(r"(?i)\b(?:spbu|pertamina|shell|bp\s*akr|vivo)\b").unwrap(), "transport"),
        (Regex::new(r"(?i)\b(?:pln|pdam|token\s*listrik)\b").unwrap(), "home"),
        (Regex::new(r"(?i)\b(?:indomaret|alfamart|alfamidi|superindo)\b").unwrap(), "shop"),
    ]
});

/// Intelligent transaction classifier combining:
/// 1. Optional user learned memory (caller can pass custom rule lookup)
/// 2. Deterministic high-priority overrides (e.g. XXI inside a mall is Fun, not Shop)
/// 3. Pre-trained on-device Multinomial Naive Bayes model
pub fn classify(merchant: &str) -> String {
    classify_with_memory(merchant, None)
}

pub fn classify_with_memory(
    merchant: &str,
    user_rules: Option<&HashMap<String, String>>,
) -> String {
    let trimmed = merchant.trim();
    if trimmed.is_empty() {
        return "other".into();
    }

    let clean_lower = trimmed.to_lowercase();
    let clean_key = Regex::new(r"[^a-z0-9 ]")
        .unwrap()
        .replace_all(&clean_lower, "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    // 1. User memory overrides (from user corrections)
    if let Some(rules) = user_rules {
        if let Some(cat) = rules.get(&clean_key) {
            return cat.clone();
        }
        for (pattern, cat) in rules {
            let pat = pattern.trim().to_lowercase();
            if !pat.is_empty() && (clean_key.contains(&pat) || pat.contains(&clean_key)) {
                return cat.clone();
            }
        }
    }

    // 2. High-priority deterministic overrides
    for (re, cat) in HIGH_PRIORITY_RULES.iter() {
        if re.is_match(trimmed) {
            return (*cat).to_string();
        }
    }

    // 3. Pre-trained Naive Bayes inference
    if let Some(predicted) = MODEL.predict(trimmed) {
        return predicted;
    }

    "other".into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_indonesian_merchants() {
        println!("Classifying PT Random Jaya Abadi: {:?}", MODEL.predict("PT Random Jaya Abadi"));
        assert_eq!(classify("Laundry Box, DYHKLT"), "home");
        assert_eq!(classify("Mie Gacoan Dipatiukur"), "food");
        assert_eq!(classify("Kopi Kenangan Rest Area KM 97"), "food");
        assert_eq!(classify("Steam Cuci Motor Rejeki"), "transport");
        assert_eq!(classify("XXI Mall Kelapa Gading"), "fun");
        assert_eq!(classify("Tagihan Listrik PLN"), "home");
        assert_eq!(classify("Alfamart Gasmin"), "shop");
        assert_eq!(classify("Beli Diamond MLBB Codashop"), "fun");
        assert_eq!(classify("Bengkel AHASS Motor Honda"), "transport");
        assert_eq!(classify("Netflix Indonesia"), "fun");
        assert_eq!(classify("Soto Ayam Ambengan"), "food");
        assert_eq!(classify("Indomaret Point Stasiun"), "shop");
        assert_eq!(classify("Tokopedia PT Tokopedia"), "shop");
        assert_eq!(classify("Warteg Kharisma Bahari"), "food");
        assert_eq!(classify("Depot Air Isi Ulang Galon Aqua"), "home");
        assert_eq!(classify("PT Random Jaya Abadi"), "other");

        // Test User Learning memory override
        let mut user_rules = HashMap::new();
        user_rules.insert("pt random jaya abadi".to_string(), "home".to_string());
        assert_eq!(
            classify_with_memory("PT Random Jaya Abadi", Some(&user_rules)),
            "home"
        );
    }
}
